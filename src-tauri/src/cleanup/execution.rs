use super::*;
use reclaim_core::cleanup::{ItemOutcome, Operation, Outcome};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

pub(crate) enum PlannedTarget {
    Filesystem(crate::scan::cleanup::Target),
    Docker(crate::docker::removal::RemovalPlan),
}
pub(super) struct PlanEntry {
    pub selection: Selection,
    pub label: String,
    pub target: PlannedTarget,
}
impl PlannedTarget {
    fn method(&self) -> Method {
        match self {
            Self::Filesystem(target) => target.method,
            Self::Docker(_) => Method::DockerImage,
        }
    }
}

struct Plan {
    id: String,
    deadline: Instant,
    entries: Vec<PlanEntry>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub plan_id: String,
    pub operation: Operation,
    pub running: bool,
    pub persistence_error: bool,
}
#[derive(Default)]
pub(super) struct ExecutionState {
    plan: Option<Plan>,
    pub execution: Option<Execution>,
}

pub(crate) struct DispatchControl<'control> {
    cancelled: &'control AtomicBool,
    deadline: Instant,
    now: &'control dyn Fn() -> Instant,
}

impl<'control> DispatchControl<'control> {
    pub(crate) fn new(
        cancelled: &'control AtomicBool,
        deadline: Instant,
        now: &'control dyn Fn() -> Instant,
    ) -> Self {
        Self {
            cancelled,
            deadline,
            now,
        }
    }

    pub(crate) fn pending_outcome(&self) -> Option<(Outcome, String)> {
        if (self.now)() >= self.deadline {
            return Some((
                Outcome::Changed,
                "Plan expired before dispatch. Create a fresh review.".into(),
            ));
        }
        if self.cancelled.load(Ordering::SeqCst) {
            return Some((
                Outcome::Skipped,
                "Cancelled before dispatch. Completed operations were not undone.".into(),
            ));
        }
        None
    }
}

impl CleanupService {
    pub(super) fn plan(
        &self,
        items: &mut [ReviewItem],
        mut entries: Vec<PlanEntry>,
    ) -> BoundaryResult<(Option<String>, Option<u64>)> {
        let mut state = lock(&self.state);
        if state
            .execution
            .as_ref()
            .is_some_and(|execution| execution.running)
        {
            return Err(ErrorCode::Busy.into());
        }
        let paths = entries
            .iter()
            .map(|entry| match &entry.target {
                PlannedTarget::Filesystem(target) => Some(target.path.clone()),
                PlannedTarget::Docker(_) => None,
            })
            .collect::<Vec<_>>();
        for item in items.iter_mut() {
            let position = entries
                .iter()
                .position(|entry| same_selection(&entry.selection, &item.selection));
            if let Some(position) = position {
                if paths.iter().enumerate().any(|(other, path)| {
                    other != position
                        && path
                            .as_ref()
                            .zip(paths[position].as_ref())
                            .is_some_and(|(ancestor, child)| child.starts_with(ancestor))
                }) {
                    item.method = None;
                    item.unavailable_reason = "Skipped: selected ancestor already covers this target. No duplicate removal.".into();
                }
            }
        }
        entries.retain(|entry| {
            items.iter().any(|item| {
                same_selection(&entry.selection, &item.selection) && item.method.is_some()
            })
        });
        let methods = items
            .iter()
            .filter_map(|item| item.method)
            .collect::<Vec<_>>();
        if methods
            .first()
            .is_some_and(|first| methods.iter().any(|method| method != first))
        {
            for item in items.iter_mut().filter(|item| item.method.is_some()) {
                item.method = None;
                item.unavailable_reason = "Review one removal method at a time. Trash and irreversible tool removal require separate confirmation.".into();
            }
            entries.clear();
        }
        if entries
            .iter()
            .map(|entry| match &entry.target {
                PlannedTarget::Filesystem(target) => target.stamps.len(),
                PlannedTarget::Docker(_) => 0,
            })
            .sum::<usize>()
            > 100_000
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
        state.plan = None;
        if entries.is_empty() {
            return Ok((None, None));
        }
        let id = uuid::Uuid::new_v4().to_string();
        state.plan = Some(Plan {
            id: id.clone(),
            deadline: Instant::now() + Duration::from_secs(120),
            entries,
        });
        Ok((Some(id), Some(now_ms() + 120_000)))
    }

    pub fn status(&self) -> Option<Execution> {
        lock(&self.state).execution.clone()
    }
    pub fn cancel(&self) {
        let mut state = lock(&self.state);
        if !state
            .execution
            .as_ref()
            .is_some_and(|execution| execution.running)
        {
            state.plan = None;
        }
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn execute(
        &self,
        scanner: &ScanService,
        docker: &DockerService,
        directory: &Path,
        plan_id: &str,
        confirmed: bool,
    ) -> BoundaryResult<Execution> {
        self.execute_with(scanner, directory, plan_id, confirmed, |target, control| {
            remove(target, docker, control)
        })
    }

    pub(crate) fn execute_with(
        &self,
        scanner: &ScanService,
        directory: &Path,
        plan_id: &str,
        confirmed: bool,
        dispatch: impl FnMut(&PlannedTarget, &DispatchControl<'_>) -> (Outcome, String),
    ) -> BoundaryResult<Execution> {
        self.execute_using(
            directory,
            plan_id,
            confirmed,
            dispatch,
            |entry| match (&entry.selection, &entry.target) {
                (
                    Selection::Filesystem {
                        generation,
                        finding_id,
                    },
                    PlannedTarget::Filesystem(expected),
                ) => scanner
                    .cleanup_target(*generation, finding_id)
                    .is_ok_and(|current| current == *expected),
                (Selection::Docker { .. }, PlannedTarget::Docker(_)) => true,
                _ => false,
            },
            Instant::now,
        )
    }

    fn execute_using(
        &self,
        directory: &Path,
        plan_id: &str,
        confirmed: bool,
        mut dispatch: impl FnMut(&PlannedTarget, &DispatchControl<'_>) -> (Outcome, String),
        mut revalidate: impl FnMut(&PlanEntry) -> bool,
        now: impl Fn() -> Instant,
    ) -> BoundaryResult<Execution> {
        let plan = {
            let mut state = lock(&self.state);
            if state
                .execution
                .as_ref()
                .is_some_and(|execution| execution.running)
            {
                return Err(ErrorCode::Busy.into());
            }
            let plan = state.plan.as_ref().ok_or(ErrorCode::InvalidRequest)?;
            if !confirmed || plan.id != plan_id || now() >= plan.deadline {
                return Err(ErrorCode::InvalidRequest.into());
            }
            let plan = state.plan.take().ok_or(ErrorCode::InvalidRequest)?;
            self.cancelled.store(false, Ordering::SeqCst);
            state.execution = Some(Execution {
                plan_id: plan.id.clone(),
                operation: Operation {
                    id: uuid::Uuid::new_v4().to_string(),
                    started_at_ms: now_ms(),
                    finished_at_ms: None,
                    volume_observations: Vec::new(),
                    items: plan
                        .entries
                        .iter()
                        .map(|entry| ItemOutcome {
                            label: entry.label.clone(),
                            method: entry.target.method(),
                            outcome: Outcome::Pending,
                            explanation: "Not yet dispatched.".into(),
                        })
                        .collect(),
                },
                running: true,
                persistence_error: false,
            });
            plan
        };
        let control = DispatchControl::new(&self.cancelled, plan.deadline, &now);
        let mut execution = self.status().ok_or(ErrorCode::Internal)?;
        let mut volumes = std::collections::BTreeMap::new();
        for entry in &plan.entries {
            if let PlannedTarget::Filesystem(target) = &entry.target {
                if let Some(parent) = target.path.parent() {
                    if let Ok(root) = reclaim_core::scan::Root::open(parent) {
                        let volume = reclaim_core::scan::volume(&root);
                        volumes
                            .entry(volume.id.clone())
                            .or_insert((parent.to_path_buf(), volume.free_bytes));
                    }
                }
            }
        }
        execution.operation.volume_observations = volumes
            .iter()
            .map(
                |(identity, (_, free))| reclaim_core::cleanup::VolumeObservation {
                    volume_id: identity.clone(),
                    before_free_bytes: free.clone(),
                    after_free_bytes: None,
                },
            )
            .collect();

        let history = {
            let _guard = lock(&self.history_lock);
            crate::cleanup_history::read(directory)
        };
        let mut history = match history {
            Ok(history) => history,
            Err(_) => {
                for item in &mut execution.operation.items {
                    item.outcome = Outcome::Skipped;
                    item.explanation = "Not dispatched: history is inaccessible or corrupt. Resolve history before a fresh review.".into();
                }
                execution.running = false;
                execution.persistence_error = true;
                lock(&self.state).execution = Some(execution.clone());
                return Ok(execution);
            }
        };
        let mut persisted = self
            .persist(directory, &mut history, &execution.operation)
            .is_ok();
        for (position, entry) in plan.entries.iter().enumerate() {
            let result = if !persisted {
                (
                    Outcome::Skipped,
                    "Not dispatched: operation history could not be persisted.".into(),
                )
            } else if let Some(outcome) = control.pending_outcome() {
                outcome
            } else if !revalidate(entry) {
                (Outcome::Changed, "Observed target, ancestor, evidence, scope or safety state changed. Rescan and create a new plan.".into())
            } else if let Some(outcome) = control.pending_outcome() {
                outcome
            } else {
                dispatch(&entry.target, &control)
            };

            execution.operation.items[position].outcome = result.0;
            execution.operation.items[position].explanation = result.1;
            if persisted {
                persisted = self
                    .persist(directory, &mut history, &execution.operation)
                    .is_ok();
            }
            execution.persistence_error = !persisted;
            lock(&self.state).execution = Some(execution.clone());
        }
        for observation in &mut execution.operation.volume_observations {
            if let Some((path, _)) = volumes.get(&observation.volume_id) {
                if let Ok(root) = reclaim_core::scan::Root::open(path) {
                    let volume = reclaim_core::scan::volume(&root);
                    if volume.id == observation.volume_id {
                        observation.after_free_bytes = volume.free_bytes;
                    }
                }
            }
        }
        execution.operation.finished_at_ms = Some(now_ms());
        execution.running = false;
        if persisted {
            persisted = self
                .persist(directory, &mut history, &execution.operation)
                .is_ok();
        }
        execution.persistence_error |= !persisted;
        lock(&self.state).execution = Some(execution.clone());
        Ok(execution)
    }

    fn persist(
        &self,
        directory: &Path,
        history: &mut History,
        operation: &Operation,
    ) -> BoundaryResult<()> {
        history
            .record(operation.clone())
            .map_err(|_| ErrorCode::InvalidRequest)?;
        let _guard = lock(&self.history_lock);
        crate::cleanup_history::write(directory, history)
    }
}

fn same_selection(left: &Selection, right: &Selection) -> bool {
    match (left, right) {
        (
            Selection::Filesystem {
                generation: left_generation,
                finding_id: left_id,
            },
            Selection::Filesystem {
                generation: right_generation,
                finding_id: right_id,
            },
        ) => left_generation == right_generation && left_id == right_id,
        (Selection::Docker { finding_id: left }, Selection::Docker { finding_id: right }) => {
            left == right
        }
        _ => false,
    }
}

pub(crate) fn trash(path: &Path) -> Result<PathBuf, ()> {
    let url = objc2_foundation::NSURL::from_file_path(path).ok_or(())?;
    let mut resulting = None;
    objc2_foundation::NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, Some(&mut resulting))
        .map_err(|_| ())?;
    resulting.and_then(|url| url.to_file_path()).ok_or(())
}

fn remove(
    target: &PlannedTarget,
    docker: &DockerService,
    control: &DispatchControl<'_>,
) -> (Outcome, String) {
    let target = match target {
        PlannedTarget::Filesystem(target) => target,
        PlannedTarget::Docker(plan) => return docker.remove_image(plan, control),
    };
    if target.method == Method::Trash {
        return match trash(&target.path) {
            Ok(_) => (Outcome::Removed, "Moved to macOS Trash. Recover through Trash / Put Back where available; no app-driven restore. Moving to Trash does not itself free the item’s storage.".into()),
            Err(_) => (Outcome::Unknown, "Trash reported an error or incomplete result. Inspect original target and macOS Trash; no permanent-delete fallback or automatic retry was attempted.".into()),
        };
    }
    let Some(repository) = &target.repository else {
        return (
            Outcome::Changed,
            "Repository unavailable; nothing dispatched.".into(),
        );
    };
    let mut child =
        match std::process::Command::new("/Library/Developer/CommandLineTools/usr/bin/git")
            .args([
                "--no-optional-locks",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "core.untrackedCache=false",
                "-c",
                "core.attributesFile=/dev/null",
            ])
            .arg("-C")
            .arg(repository)
            .args(["worktree", "remove", "--"])
            .arg(&target.path)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", "/nonexistent")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => {
                return (
                    Outcome::Failed,
                    "Git could not be started. No removal dispatched.".into(),
                )
            }
        };
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return (Outcome::Removed, "Git removed the linked worktree without force. Not recoverable through macOS Trash.".into()),
            Ok(Some(_)) | Err(_) => { let _ = child.kill(); let _ = child.wait(); return (Outcome::Unknown, "Git reported a failure; partial effects are possible. Inspect registry and target before fresh review. No force or recursive-delete fallback.".into()); }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => { let _ = child.kill(); let _ = child.wait(); return (Outcome::Unknown, "Git timed out after dispatch. Inspect target and registry; no automatic retry.".into()); }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::scan::cleanup_review_tests::{fixture, select};
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};

    fn review(
        cleanup: &CleanupService,
        scanner: &ScanService,
        generation: u32,
        names: &[&str],
    ) -> String {
        cleanup
            .review(
                scanner,
                &DockerService::default(),
                names
                    .iter()
                    .map(|name| select(scanner, generation, name))
                    .collect(),
            )
            .unwrap()
            .plan_id
            .expect("eligible fixture plan")
    }

    #[test]
    fn actual_platform_trash_dependency_and_build_preserves_source_secret_outside_and_restart_history(
    ) {
        let (temporary, scanner, generation) = fixture();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("sentinel"), b"outside").unwrap();
        let history = tempfile::tempdir().unwrap();
        let history_path = history.path().canonicalize().unwrap();
        let cleanup = CleanupService::default();
        let plan = review(&cleanup, &scanner, generation, &["node_modules", "target"]);
        let mut moved = Vec::new();
        let result = cleanup.execute_with(&scanner, &history_path, &plan, true, |target, _control| {
            let PlannedTarget::Filesystem(target) = target else { panic!("filesystem fixture only") };
            let location = trash(&target.path).expect("actual NSFileManager Trash must succeed on disposable fixture");
            assert!(location.exists());
            moved.push((target.path.clone(), location));
            (Outcome::Removed, "Moved synthetic fixture to actual macOS Trash; recover through Trash / Put Back where available.".into())
        }).unwrap();
        assert!(result
            .operation
            .items
            .iter()
            .all(|item| item.outcome == Outcome::Removed));
        assert!(!temporary.path().join("node_modules").exists());
        assert!(!temporary.path().join("target").exists());
        assert_eq!(
            fs::read(temporary.path().join("source.rs")).unwrap(),
            b"source sentinel"
        );
        assert_eq!(
            fs::read(temporary.path().join(".env")).unwrap(),
            b"secret sentinel"
        );
        assert_eq!(
            fs::read(outside.path().join("sentinel")).unwrap(),
            b"outside"
        );
        let restarted = CleanupService::default()
            .history(&history_path)
            .history
            .unwrap();
        assert_eq!(restarted.operations[0].items[0].outcome, Outcome::Removed);
        assert!(restarted.operations[0].finished_at_ms.is_some());
        assert!(!restarted.operations[0].volume_observations.is_empty());
        assert!(cleanup
            .execute(
                &scanner,
                &DockerService::default(),
                &history_path,
                &plan,
                true
            )
            .is_err());
        // Recover only the exact newly-created fixture items; never empty or enumerate user Trash.
        for (original, location) in moved {
            assert!(!original.exists());
            fs::rename(location, original).unwrap();
        }
    }

    #[test]
    fn plans_reject_forgery_expiry_unconfirmed_duplicates_and_changed_targets() {
        let (temporary, scanner, generation) = fixture();
        let history = tempfile::tempdir().unwrap();
        let directory = history.path().canonicalize().unwrap();
        let cleanup = CleanupService::default();
        let plan = review(&cleanup, &scanner, generation, &["node_modules"]);
        assert!(cleanup
            .execute_with(&scanner, &directory, "forged", true, |_, _control| panic!(
                "not dispatched"
            ))
            .is_err());
        assert!(cleanup
            .execute_with(&scanner, &directory, &plan, false, |_, _control| panic!(
                "not dispatched"
            ))
            .is_err());
        lock(&cleanup.state).plan.as_mut().unwrap().deadline =
            Instant::now() - Duration::from_secs(1);
        assert!(cleanup
            .execute_with(&scanner, &directory, &plan, true, |_, _control| panic!(
                "not dispatched"
            ))
            .is_err());
        let plan = review(&cleanup, &scanner, generation, &["node_modules"]);
        fs::write(
            temporary.path().join("node_modules/new-secret"),
            b"new data",
        )
        .unwrap();
        let result = cleanup
            .execute_with(&scanner, &directory, &plan, true, |_, _control| {
                panic!("not dispatched")
            })
            .unwrap();
        assert_eq!(result.operation.items[0].outcome, Outcome::Changed);
        assert!(cleanup
            .execute_with(&scanner, &directory, &plan, true, |_, _control| panic!(
                "not dispatched"
            ))
            .is_err());
        assert_eq!(
            fs::read(temporary.path().join("node_modules/new-secret")).unwrap(),
            b"new data"
        );
    }

    #[test]
    fn observed_symlink_or_ancestor_substitution_before_dispatch_is_changed() {
        for replace_ancestor in [false, true] {
            let (temporary, scanner, generation) = fixture();
            let history = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            fs::write(outside.path().join("sentinel"), b"outside").unwrap();
            let cleanup = CleanupService::default();
            let plan = review(&cleanup, &scanner, generation, &["node_modules"]);
            let root = temporary.path().canonicalize().unwrap();
            let target = if replace_ancestor {
                root.clone()
            } else {
                root.join("node_modules")
            };
            let moved = target.with_extension("SYNTHETIC-original");
            fs::rename(&target, &moved).unwrap();
            symlink(outside.path(), &target).unwrap();
            let result = cleanup
                .execute_with(
                    &scanner,
                    &history.path().canonicalize().unwrap(),
                    &plan,
                    true,
                    |_, _control| panic!("not dispatched"),
                )
                .unwrap();
            assert_eq!(result.operation.items[0].outcome, Outcome::Changed);
            assert_eq!(
                fs::read(outside.path().join("sentinel")).unwrap(),
                b"outside"
            );
            fs::remove_file(&target).unwrap();
            fs::rename(moved, target).unwrap();
        }
    }

    #[test]
    fn partial_failure_cancel_retry_and_duplicate_execution_do_not_widen_or_fallback() {
        let (_temporary, scanner, generation) = fixture();
        let history = tempfile::tempdir().unwrap();
        let directory = history.path().canonicalize().unwrap();
        let cleanup = CleanupService::default();
        let plan = review(&cleanup, &scanner, generation, &["node_modules", "target"]);
        let mut calls = 0;
        let result = cleanup
            .execute_with(&scanner, &directory, &plan, true, |_, _control| {
                calls += 1;
                assert!(cleanup
                    .execute_with(&scanner, &directory, &plan, true, |_, _control| panic!(
                        "duplicate"
                    ))
                    .is_err());
                assert!(cleanup.clear_history(&directory).is_err());
                cleanup.cancel();
                (
                    Outcome::Unknown,
                    "Synthetic Trash error; no permanent fallback.".into(),
                )
            })
            .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result.operation.items[0].outcome, Outcome::Unknown);
        assert_eq!(result.operation.items[1].outcome, Outcome::Skipped);
        let retry = review(&cleanup, &scanner, generation, &["target"]);
        let result = cleanup
            .execute_with(&scanner, &directory, &retry, true, |_, _control| {
                (Outcome::Failed, "Synthetic no-dispatch tool error".into())
            })
            .unwrap();
        assert_eq!(result.operation.items.len(), 1);
        assert_eq!(result.operation.items[0].outcome, Outcome::Failed);
    }

    #[test]
    fn failed_intent_or_outcome_history_persistence_stops_pending_work() {
        for failure_after_dispatch in [false, true] {
            let (_temporary, scanner, generation) = fixture();
            let history = tempfile::tempdir().unwrap();
            let directory = history.path().canonicalize().unwrap().join("records");
            fs::create_dir(&directory).unwrap();
            crate::cleanup_history::write(&directory, &History::default()).unwrap();
            let cleanup = CleanupService::default();
            let plan = review(&cleanup, &scanner, generation, &["node_modules", "target"]);
            if !failure_after_dispatch {
                fs::set_permissions(&directory, fs::Permissions::from_mode(0o500)).unwrap();
            }
            let mut calls = 0;
            let parked = directory.with_extension("parked");
            let result = cleanup
                .execute_with(&scanner, &directory, &plan, true, |_, _control| {
                    calls += 1;
                    fs::rename(&directory, &parked).unwrap();
                    fs::write(&directory, b"blocked history path").unwrap();
                    (
                        Outcome::Removed,
                        "Synthetic dispatched outcome retained in memory".into(),
                    )
                })
                .unwrap();
            assert!(result.persistence_error);
            assert_eq!(calls, usize::from(failure_after_dispatch));
            assert_eq!(result.operation.items[1].outcome, Outcome::Skipped);
            if failure_after_dispatch {
                assert_eq!(
                    CleanupService::default()
                        .history(&parked)
                        .history
                        .unwrap()
                        .operations[0]
                        .items[0]
                        .outcome,
                    Outcome::Unknown
                );
            } else {
                fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
            }
        }
    }
    #[test]
    fn linked_git_worktree_main_locked_dirty_untracked_ignored_guards_and_nonforce_removal() {
        let (temporary, scanner, _) = fixture();
        let root = temporary.path().canonicalize().unwrap();
        let main = root.join("main");
        let linked = root.join("linked");
        let git = |arguments: &[&std::ffi::OsStr]| {
            let result =
                std::process::Command::new("/Library/Developer/CommandLineTools/usr/bin/git")
                    .args(arguments)
                    .env_clear()
                    .env("HOME", "/nonexistent")
                    .env("PATH", "/usr/bin:/bin")
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .env("GIT_CONFIG_GLOBAL", "/dev/null")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .unwrap();
            assert!(result.success());
        };
        use std::ffi::OsStr;
        git(&[OsStr::new("init"), main.as_os_str()]);
        fs::write(main.join("tracked"), b"source").unwrap();
        fs::write(main.join(".gitignore"), b"ignored\n").unwrap();
        git(&[
            OsStr::new("-C"),
            main.as_os_str(),
            OsStr::new("add"),
            OsStr::new("tracked"),
            OsStr::new(".gitignore"),
        ]);
        git(&[
            OsStr::new("-C"),
            main.as_os_str(),
            OsStr::new("-c"),
            OsStr::new("user.name=Fixture"),
            OsStr::new("-c"),
            OsStr::new("user.email=fixture@example.test"),
            OsStr::new("commit"),
            OsStr::new("-m"),
            OsStr::new("synthetic"),
        ]);
        git(&[
            OsStr::new("-C"),
            main.as_os_str(),
            OsStr::new("worktree"),
            OsStr::new("add"),
            OsStr::new("-b"),
            OsStr::new("synthetic-linked"),
            linked.as_os_str(),
        ]);
        let cleanup = CleanupService::default();
        let docker = DockerService::default();
        for state in ["main", "dirty", "untracked", "ignored", "locked"] {
            if state == "dirty" {
                fs::write(linked.join("tracked"), b"changed").unwrap();
            }
            if state == "untracked" {
                fs::write(linked.join("untracked"), b"preserve").unwrap();
            }
            if state == "ignored" {
                fs::write(linked.join("ignored"), b"preserve").unwrap();
            }
            if state == "locked" {
                git(&[
                    OsStr::new("-C"),
                    main.as_os_str(),
                    OsStr::new("worktree"),
                    OsStr::new("lock"),
                    linked.as_os_str(),
                ]);
            }
            let generation = crate::scan::cleanup_review_tests::rescan(&scanner);
            let name = if state == "main" { "main" } else { "linked" };
            let report = cleanup
                .review(&scanner, &docker, vec![select(&scanner, generation, name)])
                .unwrap();
            assert!(report.plan_id.is_none(), "blocked {state}");
            assert!(linked.join("tracked").exists());
            if state == "dirty" {
                fs::write(linked.join("tracked"), b"source").unwrap();
            }
            for name in ["untracked", "ignored"] {
                if linked.join(name).exists() {
                    fs::remove_file(linked.join(name)).unwrap();
                }
            }
            if state == "locked" {
                git(&[
                    OsStr::new("-C"),
                    main.as_os_str(),
                    OsStr::new("worktree"),
                    OsStr::new("unlock"),
                    linked.as_os_str(),
                ]);
            }
        }
        let generation = crate::scan::cleanup_review_tests::rescan(&scanner);
        let plan = review(&cleanup, &scanner, generation, &["linked"]);
        let history = tempfile::tempdir().unwrap();
        let result = cleanup
            .execute(
                &scanner,
                &docker,
                &history.path().canonicalize().unwrap(),
                &plan,
                true,
            )
            .unwrap();
        assert_eq!(result.operation.items[0].method, Method::GitWorktree);
        assert_eq!(result.operation.items[0].outcome, Outcome::Removed);
        assert!(!linked.exists());
        assert_eq!(fs::read(main.join("tracked")).unwrap(), b"source");
    }
    #[test]
    fn cancellation_or_expiry_during_filesystem_revalidation_prevents_dispatch() {
        for cancel in [true, false] {
            let (temporary, scanner, generation) = fixture();
            let history = tempfile::tempdir().unwrap();
            let directory = history.path().canonicalize().unwrap();
            let cleanup = CleanupService::default();
            let plan = review(&cleanup, &scanner, generation, &["node_modules"]);
            let clock = std::cell::Cell::new(Instant::now());
            let dispatched = std::cell::Cell::new(false);
            let result = cleanup
                .execute_using(
                    &directory,
                    &plan,
                    true,
                    |_, _control| {
                        dispatched.set(true);
                        (Outcome::Removed, "unexpected dispatch".into())
                    },
                    |entry| {
                        let (
                            Selection::Filesystem {
                                generation,
                                finding_id,
                            },
                            PlannedTarget::Filesystem(expected),
                        ) = (&entry.selection, &entry.target)
                        else {
                            panic!("filesystem only")
                        };
                        let current = scanner.cleanup_target(*generation, finding_id).unwrap();
                        if cancel {
                            cleanup.cancel();
                        } else {
                            clock.set(clock.get() + Duration::from_secs(1000));
                        }
                        current == *expected
                    },
                    || clock.get(),
                )
                .unwrap();
            assert!(!dispatched.get(), "must check again after revalidation");
            let expected = if cancel {
                Outcome::Skipped
            } else {
                Outcome::Changed
            };
            assert_eq!(result.operation.items[0].outcome, expected);
            assert_eq!(
                CleanupService::default()
                    .history(&directory)
                    .history
                    .unwrap()
                    .operations[0]
                    .items[0]
                    .outcome,
                expected
            );
            assert!(temporary.path().join("node_modules").exists());
        }
    }

    pub(crate) fn assert_docker_plan_execution(
        target: crate::docker::removal::RemovalPlan,
        dispatch: impl FnMut(&PlannedTarget, &DispatchControl<'_>) -> (Outcome, String),
    ) {
        let cleanup = CleanupService::default();
        let history = tempfile::tempdir().unwrap();
        let directory = history.path().canonicalize().unwrap();
        let selection = Selection::Docker {
            finding_id: "synthetic-image-finding".into(),
        };
        let mut items = vec![ReviewItem {
            selection: selection.clone(),
            target: target.identity.clone(),
            logical_bytes: "100".into(),
            allocated_bytes: None,
            measured_at_ms: Some(now_ms()),
            artifact: None,
            unavailable_reason: String::new(),
            changed: false,
            method: Some(Method::DockerImage),
        }];
        let (plan_id, _) = cleanup
            .plan(
                &mut items,
                vec![PlanEntry {
                    selection,
                    label: target.identity.clone(),
                    target: PlannedTarget::Docker(target),
                }],
            )
            .unwrap();
        let plan_id = plan_id.unwrap();
        let result = cleanup
            .execute_with(
                &ScanService::default(),
                &directory,
                &plan_id,
                true,
                dispatch,
            )
            .unwrap();
        assert_eq!(result.operation.items[0].outcome, Outcome::Removed);
        let restarted = CleanupService::default()
            .history(&directory)
            .history
            .unwrap();
        assert_eq!(
            serde_json::to_value(&restarted.operations[0]).unwrap(),
            serde_json::to_value(&result.operation).unwrap()
        );
        assert!(cleanup
            .execute_with(
                &ScanService::default(),
                &directory,
                &plan_id,
                true,
                |_, _control| panic!("replayed plan")
            )
            .is_err());
    }

    #[test]
    fn cancellation_before_worker_claim_prevents_dispatch() {
        let (_temporary, scanner, generation) = fixture();
        let history = tempfile::tempdir().unwrap();
        let cleanup = CleanupService::default();
        let plan = review(&cleanup, &scanner, generation, &["node_modules"]);
        cleanup.cancel();
        assert!(cleanup
            .execute_with(
                &scanner,
                &history.path().canonicalize().unwrap(),
                &plan,
                true,
                |_, _control| panic!("cancelled pending request must never dispatch")
            )
            .is_err());
    }
}
