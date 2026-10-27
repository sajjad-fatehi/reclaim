use super::*;
use reclaim_core::cleanup::Outcome;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RemovalPlan {
    pub identity: String,
    endpoint: String,
    version: String,
    daemon: String,
    socket_identity: Option<(u64, u64)>,
}

impl RemovalPlan {
    pub(super) fn bind_socket(&mut self) -> Result<(), Failure> {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        let path = self
            .endpoint
            .strip_prefix("unix://")
            .ok_or(Failure::UnsupportedContext)?;
        let metadata = std::fs::metadata(path).map_err(|_| Failure::Unavailable)?;
        if !metadata.file_type().is_socket() {
            return Err(Failure::UnsupportedContext);
        }
        self.socket_identity = Some((metadata.dev(), metadata.ino()));
        Ok(())
    }
    fn socket_unchanged(&self) -> bool {
        let Some(expected) = self.socket_identity else {
            return true;
        };
        let mut current = self.clone();
        current.bind_socket().is_ok() && current.socket_identity == Some(expected)
    }
}

pub(super) fn removal_snapshot(
    transport: &impl Transport,
    home: &Path,
    identity: &str,
) -> Result<RemovalPlan, Failure> {
    let digest = identity.strip_prefix("sha256:").ok_or(Failure::Malformed)?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
    {
        return Err(Failure::Malformed);
    }
    let endpoint: String =
        serde_json::from_slice(&transport.context()?).map_err(|_| Failure::Malformed)?;
    let desktop = format!("unix://{}/.docker/run/docker.sock", home.display());
    if endpoint != "unix:///var/run/docker.sock" && endpoint != desktop {
        return Err(Failure::UnsupportedContext);
    }
    let version_bytes = transport.get(&endpoint, Request::Version)?;
    let version = docker::validate_version(&version_bytes)?;
    let version_data: serde_json::Value =
        serde_json::from_slice(&version_bytes).map_err(|_| Failure::Malformed)?;
    if !version_data["Version"]
        .as_str()
        .is_some_and(|version| version.starts_with("28."))
        || version != "1.48"
    {
        return Err(Failure::UnsupportedVersion);
    }
    let info: serde_json::Value = serde_json::from_slice(&transport.get(&endpoint, Request::Info)?)
        .map_err(|_| Failure::Malformed)?;
    let daemon = info["ID"]
        .as_str()
        .filter(|identity| !identity.is_empty() && identity.len() <= 128)
        .ok_or(Failure::Malformed)?
        .to_owned();
    if info["Driver"].as_str() != Some("overlay2")
        || info["DriverStatus"].to_string().contains("containerd")
    {
        return Err(Failure::UnsupportedEngine);
    }
    let image: serde_json::Value =
        serde_json::from_slice(&transport.get(&endpoint, Request::Image(&version, identity))?)
            .map_err(|_| Failure::Malformed)?;
    if image["Id"].as_str() != Some(identity) {
        return Err(Failure::Malformed);
    }
    for field in ["RepoTags", "RepoDigests"] {
        if !image[field].is_null() && !image[field].as_array().is_some_and(Vec::is_empty) {
            return Err(Failure::UnsupportedEngine);
        }
    }
    let containers: serde_json::Value =
        serde_json::from_slice(&transport.get(&endpoint, Request::Containers(&version))?)
            .map_err(|_| Failure::Malformed)?;
    let containers = containers.as_array().ok_or(Failure::Malformed)?;
    if containers.len() > 4096
        || containers.iter().any(|container| {
            container["ImageID"]
                .as_str()
                .is_none_or(|image| image == identity)
        })
    {
        return Err(Failure::UnsupportedEngine);
    }
    let usage = docker::parse_inventory(&transport.get(&endpoint, Request::Usage(&version))?)?;
    if !usage.items.iter().any(|item| {
        item.kind == reclaim_core::docker::ResourceKind::Image
            && item.identity == identity
            && item.references == Some(0)
    }) {
        return Err(Failure::UnsupportedEngine);
    }
    Ok(RemovalPlan {
        identity: identity.into(),
        endpoint,
        version,
        daemon,
        socket_identity: None,
    })
}

pub(super) fn remove_checked(
    transport: &impl Transport,
    home: &Path,
    expected: &RemovalPlan,
    control: &crate::cleanup::execution::DispatchControl<'_>,
) -> (Outcome, String) {
    let mut comparison = expected.clone();
    comparison.socket_identity = None;
    match removal_snapshot(transport, home, &expected.identity) {
        Ok(current) if current == comparison && expected.socket_unchanged() => {}
        _ => return (Outcome::Changed, "Docker context, endpoint, daemon, image, version or references changed. No removal dispatched; create a fresh plan.".into()),
    }
    if let Some(outcome) = control.pending_outcome() {
        return outcome;
    }
    let result = transport.get(
        &expected.endpoint,
        Request::DeleteImage(&expected.version, &expected.identity),
    );
    if let Ok(bytes) = result {
        if let Ok(serde_json::Value::Array(records)) =
            serde_json::from_slice::<serde_json::Value>(&bytes)
        {
            if !records.is_empty()
                && records.len() <= 4096
                && records.iter().all(|record| {
                    record.as_object().is_some_and(|object| {
                        object.len() == 1
                            && (record["Deleted"].is_string() || record["Untagged"].is_string())
                    })
                })
                && records
                    .iter()
                    .any(|record| record["Deleted"].as_str() == Some(expected.identity.as_str()))
            {
                return (Outcome::Removed, "Docker confirmed exact image removal without force or parent pruning. Layers may be shared. Not recoverable through Trash; rebuilding/downloading may be required.".into());
            }
        }
    }
    (Outcome::Unknown, "Docker removal response was unsuccessful, malformed or ambiguous. Partial untagging/removal may have occurred, including on conflict. Inspect daemon before a fresh review; no automatic retry or endpoint fallback.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    struct Stub {
        calls: RefCell<Vec<String>>,
        daemon: RefCell<String>,
        endpoint: RefCell<String>,
        referenced: Cell<bool>,
        tagged: Cell<bool>,
        failure: Cell<bool>,
        malformed: Cell<bool>,
        after_usage: RefCell<Option<Box<dyn FnOnce()>>>,
    }
    fn identity() -> String {
        format!("sha256:{}", "a".repeat(64))
    }
    fn stub() -> Stub {
        Stub {
            calls: RefCell::default(),
            daemon: RefCell::new("synthetic-daemon".into()),
            endpoint: RefCell::new("unix:///var/run/docker.sock".into()),
            referenced: Cell::new(false),
            tagged: Cell::new(false),
            failure: Cell::new(false),
            malformed: Cell::new(false),
            after_usage: RefCell::new(None),
        }
    }
    impl Transport for Stub {
        fn context(&self) -> Result<Vec<u8>, Failure> {
            self.calls.borrow_mut().push("context".into());
            Ok(serde_json::to_vec(&*self.endpoint.borrow()).unwrap())
        }
        fn get(&self, endpoint: &str, request: Request) -> Result<Vec<u8>, Failure> {
            assert_eq!(endpoint, *self.endpoint.borrow());
            let data = match request {
                Request::Version => {
                    self.calls.borrow_mut().push("GET /version".into());
                    serde_json::json!({"Platform":{"Name":"Docker Engine - Community"},"Version":"28.0.0","ApiVersion":"1.48"})
                }
                Request::Info => {
                    self.calls.borrow_mut().push("GET /info".into());
                    serde_json::json!({"ID":*self.daemon.borrow(),"Driver":"overlay2","DriverStatus":[]})
                }
                Request::Image(version, selected) => {
                    assert_eq!(selected, identity());
                    assert_eq!(version, "1.48");
                    self.calls.borrow_mut().push("GET exact image".into());
                    serde_json::json!({"Id":identity(),"RepoTags": if self.tagged.get() { vec!["synthetic:tag"] } else { vec![] }, "RepoDigests":[]})
                }
                Request::Containers(version) => {
                    assert_eq!(version, "1.48");
                    self.calls
                        .borrow_mut()
                        .push("GET /containers/json?all=true".into());
                    if self.referenced.get() {
                        serde_json::json!([{"ImageID":identity(),"State":"exited"}])
                    } else {
                        serde_json::json!([])
                    }
                }
                Request::Usage(version) => {
                    assert_eq!(version, "1.48");
                    self.calls.borrow_mut().push("GET /system/df".into());
                    if let Some(action) = self.after_usage.borrow_mut().take() {
                        action();
                    }
                    serde_json::json!({"Images":[{"Id":identity(),"Size":100,"Containers":0,"SharedSize":0}],"Containers":[],"Volumes":[],"BuildCache":[]})
                }
                Request::DeleteImage(version, selected) => {
                    assert_eq!(version, "1.48");
                    assert_eq!(selected, identity());
                    self.calls.borrow_mut().push(format!(
                        "DELETE /v{version}/images/{selected}?force=false&noprune=true"
                    ));
                    if self.failure.get() {
                        return Err(Failure::DaemonUnavailable);
                    }
                    if self.malformed.get() {
                        return Ok(b"partial response".to_vec());
                    }
                    serde_json::json!([{"Deleted":identity()},{"Deleted":"synthetic-layer-chain"}])
                }
            };
            Ok(serde_json::to_vec(&data).unwrap())
        }
    }
    #[test]
    fn cancellation_or_expiry_during_docker_fresh_checks_prevents_delete() {
        for cancel in [true, false] {
            let fixture = stub();
            let home = Path::new("/SYNTHETIC");
            let plan = removal_snapshot(&fixture, home, &identity()).unwrap();
            let cancelled = std::sync::Arc::new(AtomicBool::new(false));
            let clock = std::rc::Rc::new(Cell::new(Instant::now()));
            let deadline = clock.get() + Duration::from_secs(120);
            let changed_cancelled = cancelled.clone();
            let changed_clock = clock.clone();
            *fixture.after_usage.borrow_mut() = Some(Box::new(move || {
                if cancel {
                    changed_cancelled.store(true, Ordering::SeqCst);
                } else {
                    changed_clock.set(deadline);
                }
            }));
            let now = || clock.get();
            let control =
                crate::cleanup::execution::DispatchControl::new(&cancelled, deadline, &now);
            let result = remove_checked(&fixture, home, &plan, &control);
            assert!(
                !fixture
                    .calls
                    .borrow()
                    .iter()
                    .any(|call| call.starts_with("DELETE ")),
                "must check again after fresh Docker observations"
            );
            assert_eq!(
                result.0,
                if cancel {
                    Outcome::Skipped
                } else {
                    Outcome::Changed
                }
            );
        }
    }

    #[test]
    fn native_plan_executes_exact_docker_removal_and_persists_history() {
        let fixture = stub();
        let home = Path::new("/SYNTHETIC");
        let plan = removal_snapshot(&fixture, home, &identity()).unwrap();
        crate::cleanup::execution::tests::assert_docker_plan_execution(plan, |target, control| {
            let crate::cleanup::execution::PlannedTarget::Docker(plan) = target else {
                panic!("Docker only")
            };
            remove_checked(&fixture, home, plan, control)
        });
        assert_eq!(
            fixture
                .calls
                .borrow()
                .iter()
                .filter(|call| call.starts_with("DELETE "))
                .count(),
            1
        );
    }

    #[test]
    fn exact_route_fresh_context_daemon_references_and_no_force_prune_retry() {
        let fixture = stub();
        let plan = removal_snapshot(&fixture, Path::new("/SYNTHETIC"), &identity()).unwrap();
        assert_eq!(
            remove_checked(
                &fixture,
                Path::new("/SYNTHETIC"),
                &plan,
                &crate::cleanup::execution::DispatchControl::new(
                    &AtomicBool::new(false),
                    Instant::now() + Duration::from_secs(120),
                    &Instant::now
                )
            )
            .0,
            Outcome::Removed
        );
        assert_eq!(
            fixture
                .calls
                .borrow()
                .iter()
                .filter(|call| call.starts_with("DELETE "))
                .count(),
            1
        );
        for change in ["context", "daemon", "reference", "tag"] {
            let fixture = stub();
            let plan = removal_snapshot(&fixture, Path::new("/SYNTHETIC"), &identity()).unwrap();
            match change {
                "context" => *fixture.endpoint.borrow_mut() = "tcp://remote:2375".into(),
                "daemon" => *fixture.daemon.borrow_mut() = "changed-daemon".into(),
                "reference" => fixture.referenced.set(true),
                _ => fixture.tagged.set(true),
            }
            assert_eq!(
                remove_checked(
                    &fixture,
                    Path::new("/SYNTHETIC"),
                    &plan,
                    &crate::cleanup::execution::DispatchControl::new(
                        &AtomicBool::new(false),
                        Instant::now() + Duration::from_secs(120),
                        &Instant::now
                    )
                )
                .0,
                Outcome::Changed
            );
            assert!(!fixture
                .calls
                .borrow()
                .iter()
                .any(|call| call.starts_with("DELETE ")));
        }
        let fixture = stub();
        assert!(removal_snapshot(
            &fixture,
            Path::new("/SYNTHETIC"),
            "image-tag-or-forged/path"
        )
        .is_err());
        assert!(fixture.calls.borrow().is_empty());
    }
    #[test]
    fn dispatched_error_or_partial_response_is_unknown_never_unchanged_or_retried() {
        for malformed in [false, true] {
            let fixture = stub();
            let plan = removal_snapshot(&fixture, Path::new("/SYNTHETIC"), &identity()).unwrap();
            fixture.failure.set(!malformed);
            fixture.malformed.set(malformed);
            let result = remove_checked(
                &fixture,
                Path::new("/SYNTHETIC"),
                &plan,
                &crate::cleanup::execution::DispatchControl::new(
                    &AtomicBool::new(false),
                    Instant::now() + Duration::from_secs(120),
                    &Instant::now,
                ),
            );
            assert_eq!(result.0, Outcome::Unknown);
            assert!(result.1.contains("Partial"));
            assert_eq!(
                fixture
                    .calls
                    .borrow()
                    .iter()
                    .filter(|call| call.starts_with("DELETE "))
                    .count(),
                1
            );
        }
    }
}
