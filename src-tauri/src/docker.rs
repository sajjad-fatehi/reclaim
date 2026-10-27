//! Fixed native Docker routes only. No generic process, endpoint or shell IPC.
use reclaim_core::docker::{self, Failure, Inventory};
use serde::Serialize;
use std::io::Read;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{State, WebviewWindow};

const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
const BUDGET: Duration = Duration::from_secs(20);

#[derive(Default)]
pub struct DockerService {
    busy: AtomicBool,
    cancelled: AtomicBool,
    observations: Mutex<Option<(u64, Vec<reclaim_core::docker::Item>)>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub measured_at_ms: String,
    pub result: Result<Inventory, Failure>,
}

trait Transport {
    fn context(&self) -> Result<Vec<u8>, Failure>;
    fn get(&self, endpoint: &str, request: Request) -> Result<Vec<u8>, Failure>;
}

#[derive(Clone, Copy)]
enum Request<'version> {
    Version,
    Usage(&'version str),
    Info,
    Image(&'version str, &'version str),
    Containers(&'version str),
    DeleteImage(&'version str, &'version str),
}

struct LocalTransport<'service> {
    service: &'service DockerService,
    started: Instant,
    cli: PathBuf,
    home: PathBuf,
}

impl LocalTransport<'_> {
    fn run(&self, executable: &Path, arguments: &[&str]) -> Result<Vec<u8>, Failure> {
        self.check()?;
        let mut command = Command::new(executable);
        command
            .args(arguments)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|_| Failure::Unavailable)?;
        let outcome = (|| {
            let mut output = child.stdout.take().ok_or(Failure::Unavailable)?;
            // Descendants can retain stdout after CLI exit; pipe waits must stay cancellable.
            let flags =
                rustix::fs::fcntl_getfl(output.as_fd()).map_err(|_| Failure::Unavailable)?;
            rustix::fs::fcntl_setfl(output.as_fd(), flags | rustix::fs::OFlags::NONBLOCK)
                .map_err(|_| Failure::Unavailable)?;
            let mut bytes = Vec::new();
            let mut buffer = [0_u8; 65_536];
            let mut status = None;
            let mut eof = false;
            loop {
                self.check()?;
                let mut received = false;
                if !eof {
                    let remaining = (OUTPUT_LIMIT + 1 - bytes.len()).min(buffer.len());
                    match output.read(&mut buffer[..remaining]) {
                        Ok(0) => eof = true,
                        Ok(count) => {
                            bytes.extend_from_slice(&buffer[..count]);
                            if bytes.len() > OUTPUT_LIMIT {
                                return Err(Failure::Limited);
                            }
                            received = true;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => return Err(Failure::Unavailable),
                    }
                }
                if status.is_none() {
                    status = child.try_wait().map_err(|_| Failure::Unavailable)?;
                }
                if eof {
                    if let Some(status) = status {
                        return Ok((bytes, status));
                    }
                }
                if !received {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        })();
        if outcome.is_err() {
            let _ = child.kill();
        }
        let _ = child.wait();
        let (bytes, status) = outcome?;
        if !status.success() {
            return Err(match status.code() {
                Some(28) if executable == Path::new("/usr/bin/curl") => Failure::Timeout,
                Some(63) if executable == Path::new("/usr/bin/curl") => Failure::Limited,
                _ => Failure::DaemonUnavailable,
            });
        }
        Ok(bytes)
    }

    fn check(&self) -> Result<(), Failure> {
        if self.service.cancelled.load(Ordering::Relaxed) {
            return Err(Failure::Cancelled);
        }
        if self.started.elapsed() >= BUDGET {
            return Err(Failure::Timeout);
        }
        Ok(())
    }
}

impl Transport for LocalTransport<'_> {
    fn context(&self) -> Result<Vec<u8>, Failure> {
        let version = self.run(&self.cli, &["--version"])?;
        let version = std::str::from_utf8(&version).map_err(|_| Failure::Malformed)?;
        let Some(version) = version.strip_prefix("Docker version ") else {
            return Err(Failure::UnsupportedEngine);
        };
        let major = version
            .split('.')
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or(Failure::Malformed)?;
        if !(24..=29).contains(&major) {
            return Err(Failure::UnsupportedVersion);
        }
        let context = std::env::var("DOCKER_CONTEXT").unwrap_or_default();
        if !context.is_empty() && context != "default" && context != "desktop-linux" {
            return Err(Failure::UnsupportedContext);
        }
        if std::env::var_os("DOCKER_HOST").is_some() || std::env::var_os("DOCKER_CONFIG").is_some()
        {
            return Err(Failure::UnsupportedContext);
        }
        let context = if context.is_empty() {
            self.run(&self.cli, &["context", "show"])?
        } else {
            context.into_bytes()
        };
        let context = std::str::from_utf8(&context)
            .map_err(|_| Failure::Malformed)?
            .trim();
        if context != "default" && context != "desktop-linux" {
            return Err(Failure::UnsupportedContext);
        }
        self.run(
            &self.cli,
            &[
                "context",
                "inspect",
                context,
                "--format",
                "{{json .Endpoints.docker.Host}}",
            ],
        )
    }

    fn get(&self, endpoint: &str, request: Request) -> Result<Vec<u8>, Failure> {
        let socket = endpoint
            .strip_prefix("unix://")
            .ok_or(Failure::UnsupportedContext)?;
        let route = match request {
            Request::Version => "/version".into(),
            Request::Info => "/info".into(),
            Request::Image(version, identity) => format!("/v{version}/images/{identity}/json"),
            Request::Containers(version) => format!("/v{version}/containers/json?all=true"),
            Request::DeleteImage(version, identity) => {
                format!("/v{version}/images/{identity}?force=false&noprune=true")
            }
            Request::Usage(version) => format!("/v{version}/system/df"),
        };
        self.run(
            Path::new("/usr/bin/curl"),
            &[
                "--disable",
                "--silent",
                "--fail",
                "--noproxy",
                "*",
                "--proxy",
                "",
                "--connect-timeout",
                "2",
                "--max-time",
                "5",
                "--max-filesize",
                "8388608",
                "--request",
                if matches!(request, Request::DeleteImage(_, _)) {
                    "DELETE"
                } else {
                    "GET"
                },
                "--retry",
                "0",
                "--unix-socket",
                socket,
                &format!("http://localhost{route}"),
            ],
        )
    }
}

fn collect(transport: &impl Transport, home: &Path) -> Result<Inventory, Failure> {
    let endpoint: String =
        serde_json::from_slice(&transport.context()?).map_err(|_| Failure::Malformed)?;
    let desktop = format!("unix://{}/.docker/run/docker.sock", home.display());
    if endpoint != "unix:///var/run/docker.sock" && endpoint != desktop {
        return Err(Failure::UnsupportedContext);
    }
    let version = docker::validate_version(&transport.get(&endpoint, Request::Version)?)?;
    docker::parse_inventory(&transport.get(&endpoint, Request::Usage(&version))?)
}

impl DockerService {
    pub fn inventory(&self) -> Result<Snapshot, Failure> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(Failure::Busy);
        }
        self.cancelled.store(false, Ordering::SeqCst);
        let result = self.with_local(|transport, home| collect(transport, home));
        let measured_at_ms = reclaim_core::scan::now_ms();
        *reclaim_core::scan::lock(&self.observations) = result
            .as_ref()
            .ok()
            .map(|inventory| (measured_at_ms, inventory.items.clone()));
        self.busy.store(false, Ordering::SeqCst);
        Ok(Snapshot {
            measured_at_ms: measured_at_ms.to_string(),
            result,
        })
    }

    fn with_local<Output>(
        &self,
        action: impl FnOnce(&LocalTransport<'_>, &Path) -> Result<Output, Failure>,
    ) -> Result<Output, Failure> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(Failure::Unavailable)?;
        let cli = [
            "/usr/local/bin/docker",
            "/opt/homebrew/bin/docker",
            "/Applications/Docker.app/Contents/Resources/bin/docker",
        ]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .ok_or(Failure::Unavailable)?;
        action(
            &LocalTransport {
                service: self,
                started: Instant::now(),
                cli,
                home: home.clone(),
            },
            &home,
        )
    }

    pub(crate) fn prepare_removal(&self, finding_id: &str) -> Result<RemovalPlan, Failure> {
        let (_, item) = self.observed_item(finding_id).ok_or(Failure::Malformed)?;
        if item.kind != reclaim_core::docker::ResourceKind::Image || item.references != Some(0) {
            return Err(Failure::UnsupportedEngine);
        }
        if self.busy.swap(true, Ordering::SeqCst) {
            return Err(Failure::Busy);
        }
        self.cancelled.store(false, Ordering::SeqCst);
        let result = self.with_local(|transport, home| {
            let mut plan = removal_snapshot(transport, home, &item.identity)?;
            plan.bind_socket()?;
            Ok(plan)
        });
        self.busy.store(false, Ordering::SeqCst);
        result
    }

    pub(crate) fn remove_image(
        &self,
        expected: &RemovalPlan,
        control: &crate::cleanup::execution::DispatchControl<'_>,
    ) -> (reclaim_core::cleanup::Outcome, String) {
        use reclaim_core::cleanup::Outcome;
        if self.busy.swap(true, Ordering::SeqCst) {
            return (
                Outcome::Skipped,
                "Docker busy. No removal dispatched.".into(),
            );
        }
        self.cancelled.store(false, Ordering::SeqCst);
        let result = self
            .with_local(|transport, home| Ok(remove_checked(transport, home, expected, control)));
        self.busy.store(false, Ordering::SeqCst);
        result.unwrap_or((
            Outcome::Changed,
            "Local Docker discovery unavailable. No removal dispatched.".into(),
        ))
    }

    pub(crate) fn observed_item(
        &self,
        finding_id: &str,
    ) -> Option<(u64, reclaim_core::docker::Item)> {
        let observations = reclaim_core::scan::lock(&self.observations);
        let (measured_at_ms, items) = observations.as_ref()?;
        items
            .iter()
            .find(|item| item.artifact.finding_id == finding_id)
            .cloned()
            .map(|item| (*measured_at_ms, item))
    }
}

#[tauri::command]
pub async fn docker_inventory<Runtime: tauri::Runtime>(
    window: WebviewWindow<Runtime>,
    service: State<'_, std::sync::Arc<DockerService>>,
) -> Result<Snapshot, Failure> {
    if window.label() != "main" {
        return Err(Failure::AccessDenied);
    }
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.inventory())
        .await
        .map_err(|_| Failure::Unavailable)?
}

#[tauri::command]
pub fn docker_cancel<Runtime: tauri::Runtime>(
    window: WebviewWindow<Runtime>,
    service: State<'_, std::sync::Arc<DockerService>>,
) -> Result<(), Failure> {
    if window.label() != "main" {
        return Err(Failure::AccessDenied);
    }
    service.cancelled.store(true, Ordering::SeqCst);
    Ok(())
}

pub(crate) mod removal;
use removal::{removal_snapshot, remove_checked, RemovalPlan};

#[cfg(test)]
mod tests;
