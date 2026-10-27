use super::*;
use std::cell::RefCell;

struct Fixture {
    endpoint: Vec<u8>,
    failure: Option<Failure>,
    calls: RefCell<Vec<String>>,
}
impl Transport for Fixture {
    fn context(&self) -> Result<Vec<u8>, Failure> {
        self.calls.borrow_mut().push("context".into());
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        Ok(self.endpoint.clone())
    }
    fn get(&self, endpoint: &str, request: Request) -> Result<Vec<u8>, Failure> {
        assert_eq!(endpoint, "unix:///var/run/docker.sock");
        match request {
            Request::Info
            | Request::Image(_, _)
            | Request::Containers(_)
            | Request::DeleteImage(_, _) => Err(Failure::UnsupportedEngine),
            Request::Version => {
                self.calls.borrow_mut().push("GET /version".into());
                Ok(br#"{"Platform":{"Name":"Docker Engine - Community"},"Version":"28.0.0","ApiVersion":"1.48"}"#.to_vec())
            }
            Request::Usage(version) => {
                assert_eq!(version, "1.48");
                self.calls.borrow_mut().push("GET /v1.48/system/df".into());
                Ok(br#"{"Images":[],"Containers":[],"Volumes":[],"BuildCache":[]}"#.to_vec())
            }
        }
    }
}
fn fixture(endpoint: &str) -> Fixture {
    Fixture {
        endpoint: serde_json::to_vec(endpoint).unwrap(),
        failure: None,
        calls: RefCell::default(),
    }
}

#[test]
fn cleanup_reviews_only_native_observations_and_never_calls_docker_mutation() {
    let service = DockerService::default();
    let identity = format!("sha256:{}", "a".repeat(64));
    let payload = serde_json::to_vec(&serde_json::json!({
        "Images":[{"Id":identity,"Size":123,"Containers":0,"SharedSize":0}],
        "Containers":[],"Volumes":[],"BuildCache":[]
    }))
    .unwrap();
    let inventory = docker::parse_inventory(&payload).unwrap();
    let finding_id = inventory.items[0].artifact.finding_id.clone();
    *reclaim_core::scan::lock(&service.observations) = Some((1234, inventory.items));
    let (observed_at, observed) = service.observed_item(&finding_id).unwrap();
    assert_eq!(observed.identity, identity);
    assert_eq!(observed_at, 1234);
    assert!(service.observed_item("forged").is_none());
    // Review only reads the native cache; no process, HTTP DELETE, or tool transport is used.
    assert!(!service.busy.load(Ordering::SeqCst));
}

#[test]
fn fixture_inventory_uses_only_verified_local_gets() {
    let fixture = fixture("unix:///var/run/docker.sock");
    assert!(collect(&fixture, Path::new("/synthetic"))
        .unwrap()
        .items
        .is_empty());
    assert_eq!(
        *fixture.calls.borrow(),
        ["context", "GET /version", "GET /v1.48/system/df"]
    );
}

#[test]
fn remote_and_alternative_endpoints_never_receive_requests() {
    for endpoint in [
        "ssh://host",
        "tcp://127.0.0.1:2375",
        "unix:///tmp/podman.sock",
        "unix:///var/run/docker.sock/../other",
    ] {
        let fixture = fixture(endpoint);
        assert_eq!(
            collect(&fixture, Path::new("/synthetic")).unwrap_err(),
            Failure::UnsupportedContext
        );
        assert_eq!(*fixture.calls.borrow(), ["context"]);
    }
}

#[test]
fn failures_remain_distinct_and_do_not_start_daemons() {
    for failure in [
        Failure::Unavailable,
        Failure::DaemonUnavailable,
        Failure::Timeout,
        Failure::Cancelled,
        Failure::Malformed,
        Failure::Limited,
    ] {
        let mut fixture = fixture("unix:///var/run/docker.sock");
        fixture.failure = Some(failure);
        assert_eq!(
            collect(&fixture, Path::new("/synthetic")).unwrap_err(),
            failure
        );
        assert_eq!(*fixture.calls.borrow(), ["context"]);
    }
}

#[test]
fn process_budget_and_cancellation_kill_and_reap_only_fixture_processes() {
    let service = DockerService::default();
    let transport = LocalTransport {
        service: &service,
        started: Instant::now(),
        cli: PathBuf::new(),
        home: PathBuf::from("/synthetic"),
    };
    assert_eq!(
        transport
            .run(Path::new("/nonexistent/docker"), &[])
            .unwrap_err(),
        Failure::Unavailable
    );
    service.cancelled.store(true, Ordering::Relaxed);
    assert_eq!(
        transport.run(Path::new("/usr/bin/true"), &[]).unwrap_err(),
        Failure::Cancelled
    );
    service.cancelled.store(false, Ordering::Relaxed);
    let expired = LocalTransport {
        started: Instant::now() - BUDGET,
        ..transport
    };
    assert_eq!(
        expired.run(Path::new("/usr/bin/true"), &[]).unwrap_err(),
        Failure::Timeout
    );
    service.busy.store(true, Ordering::SeqCst);
    assert_eq!(service.inventory().err(), Some(Failure::Busy));
}

#[test]
fn running_fixture_process_is_cancelled_and_output_is_bounded() {
    let service = DockerService::default();
    let transport = LocalTransport {
        service: &service,
        started: Instant::now(),
        cli: PathBuf::new(),
        home: PathBuf::from("/synthetic"),
    };
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(80));
            service.cancelled.store(true, Ordering::Relaxed);
        });
        assert_eq!(
            transport.run(Path::new("/bin/sleep"), &["10"]).unwrap_err(),
            Failure::Cancelled
        );
    });
    service.cancelled.store(false, Ordering::Relaxed);
    let expiring = LocalTransport {
        started: Instant::now() - BUDGET + Duration::from_millis(80),
        ..transport
    };
    assert_eq!(
        expiring.run(Path::new("/bin/sleep"), &["10"]).unwrap_err(),
        Failure::Timeout
    );
    let transport = LocalTransport {
        started: Instant::now(),
        ..expiring
    };
    assert_eq!(
        transport.run(Path::new("/usr/bin/yes"), &[]).unwrap_err(),
        Failure::Limited
    );
}

#[test]
fn ipc_rejects_other_windows_and_serializes_busy_without_discovery() {
    use tauri::test::{get_ipc_response, mock_builder, MockRuntime, INVOKE_KEY};
    let service = std::sync::Arc::new(DockerService::default());
    service.busy.store(true, Ordering::SeqCst);
    let mut context = crate::test_context();
    context.config_mut().app.windows.clear();
    let app = mock_builder()
        .manage(service.clone())
        .invoke_handler(tauri::generate_handler![docker_inventory, docker_cancel])
        .build(context)
        .unwrap();
    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let other = tauri::WebviewWindowBuilder::new(&app, "other", Default::default())
        .build()
        .unwrap();
    fn invoke(
        window: &tauri::WebviewWindow<MockRuntime>,
        command: &str,
    ) -> Result<serde_json::Value, serde_json::Value> {
        get_ipc_response(
            window,
            tauri::webview::InvokeRequest {
                cmd: command.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: window.url().unwrap(),
                body: tauri::ipc::InvokeBody::Json(serde_json::json!({})),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.into(),
            },
        )
        .map(|body| body.deserialize().unwrap())
    }
    assert_eq!(invoke(&main, "docker_inventory").unwrap_err(), "busy");
    assert!(invoke(&other, "docker_cancel").is_err());
    assert!(invoke(&other, "docker_inventory").is_err());
    assert!(!service.cancelled.load(Ordering::SeqCst));
    assert!(invoke(&main, "docker_cancel").is_ok());
    assert!(service.cancelled.load(Ordering::SeqCst));
}

#[test]
fn unix_transport_sends_fixed_get_without_credentials_or_network_fetch() {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("synthetic.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        let (mut connection, _) = listener.accept().unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = [0; 4096];
        let count = connection.read(&mut request).unwrap();
        let request = std::str::from_utf8(&request[..count]).unwrap();
        assert!(request.starts_with("GET /version HTTP/1.1\r\n"));
        assert!(!request.to_ascii_lowercase().contains("authorization"));
        let body = br#"{"synthetic":true}"#;
        write!(
            connection,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        connection.write_all(body).unwrap();
    });
    let service = DockerService::default();
    let transport = LocalTransport {
        service: &service,
        started: Instant::now(),
        cli: PathBuf::new(),
        home: temporary.path().to_owned(),
    };
    let body = transport
        .get(&format!("unix://{}", socket.display()), Request::Version)
        .unwrap();
    assert_eq!(body, br#"{"synthetic":true}"#);
    server.join().unwrap();
}

struct InheritedStdoutFixture {
    directory: tempfile::TempDir,
    executable: PathBuf,
}

impl InheritedStdoutFixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("synthetic-docker");
        std::fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s' \"$$\" > \"$HOME/parent.pid\"\n/bin/sleep 5 &\nprintf '%s' \"$!\" > \"$HOME/holder.pid\"\nprintf synthetic\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            directory,
            executable,
        }
    }

    fn process(&self, name: &str) -> Option<rustix::process::Pid> {
        std::fs::read_to_string(self.directory.path().join(name))
            .ok()
            .and_then(|value| value.parse::<i32>().ok())
            .and_then(rustix::process::Pid::from_raw)
    }

    fn parent_exited(&self) -> bool {
        self.process("parent.pid").is_some_and(|parent| {
            rustix::process::test_kill_process(parent) == Err(rustix::io::Errno::SRCH)
        })
    }

    fn cleanup(&self) -> bool {
        let Some(holder) = self.process("holder.pid") else {
            return true;
        };
        let _ = rustix::process::kill_process(holder, rustix::process::Signal::KILL);
        let started = Instant::now();
        while rustix::process::test_kill_process(holder).is_ok() {
            if started.elapsed() >= Duration::from_secs(2) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = std::fs::remove_file(self.directory.path().join("holder.pid"));
        true
    }
}

impl Drop for InheritedStdoutFixture {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn assert_inherited_stdout_terminates(cancel: bool) {
    let fixture = InheritedStdoutFixture::new();
    let service = DockerService::default();
    let transport = LocalTransport {
        service: &service,
        started: if cancel {
            Instant::now()
        } else {
            Instant::now() - BUDGET + Duration::from_secs(1)
        },
        cli: fixture.executable.clone(),
        home: fixture.directory.path().to_owned(),
    };
    let cancelled_at = std::sync::Mutex::new(None);
    let started = Instant::now();
    let result = std::thread::scope(|scope| {
        if cancel {
            scope.spawn(|| {
                let started = Instant::now();
                while !fixture.parent_exited() {
                    assert!(
                        started.elapsed() < Duration::from_secs(2),
                        "wrapper did not exit"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                assert!(fixture.process("holder.pid").is_some());
                *cancelled_at.lock().unwrap() = Some(Instant::now());
                service.cancelled.store(true, Ordering::Relaxed);
            });
        }
        transport.run(&fixture.executable, &[])
    });
    let elapsed = started.elapsed();
    let cancellation_elapsed = cancelled_at
        .lock()
        .unwrap()
        .map(|instant| instant.elapsed());
    let launched_holder = fixture.process("holder.pid").is_some();
    let parent_exited = fixture.parent_exited();
    assert!(fixture.cleanup(), "fixture holder not reaped");
    assert!(launched_holder, "wrapper did not launch holder: {result:?}");
    assert!(parent_exited, "direct wrapper child was not reaped");
    assert_eq!(
        result.unwrap_err(),
        if cancel {
            Failure::Cancelled
        } else {
            Failure::Timeout
        }
    );
    if let Some(elapsed) = cancellation_elapsed {
        assert!(
            elapsed < Duration::from_millis(250),
            "cancellation exceeded bound: {elapsed:?}"
        );
    }
    assert!(
        elapsed < Duration::from_secs(2),
        "command exceeded bound: {elapsed:?}"
    );
}

#[test]
fn inherited_stdout_after_parent_exit_respects_deadline() {
    assert_inherited_stdout_terminates(false);
}

#[test]
fn inherited_stdout_after_parent_exit_respects_cancellation() {
    assert_inherited_stdout_terminates(true);
}
