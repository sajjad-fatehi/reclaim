//! Native-owned cleanup authority; explicit plans and practical immediate revalidation.
use crate::docker::DockerService;
use crate::scan::{authorize, BoundaryResult, ErrorCode, ScanService};
use reclaim_core::cleanup::{filesystem_unavailability, History, Method, MAX_ITEMS};
use std::sync::atomic::AtomicBool;
pub(crate) mod execution;
pub use execution::Execution;
use execution::{ExecutionState, PlanEntry, PlannedTarget};
use reclaim_core::recognition::Artifact;
use reclaim_core::scan::{lock, now_ms};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, Runtime, State, WebviewWindow};

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    Filesystem {
        generation: u32,
        #[serde(rename = "findingId")]
        finding_id: String,
    },
    Docker {
        #[serde(rename = "findingId")]
        finding_id: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewItem {
    pub selection: Selection,
    pub target: String,
    pub logical_bytes: String,
    pub allocated_bytes: Option<String>,
    pub measured_at_ms: Option<u64>,
    pub artifact: Option<Artifact>,
    pub unavailable_reason: String,
    pub changed: bool,
    pub method: Option<Method>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub checked_at_ms: u64,
    pub items: Vec<ReviewItem>,
    pub plan_id: Option<String>,
    pub expires_at_ms: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResponse {
    pub history: Option<History>,
    pub unavailable: bool,
}

#[derive(Default)]
pub struct CleanupService {
    history_lock: Mutex<()>,
    state: Mutex<ExecutionState>,
    cancelled: AtomicBool,
}

impl CleanupService {
    pub fn review(
        &self,
        scanner: &ScanService,
        docker: &DockerService,
        selections: Vec<Selection>,
    ) -> BoundaryResult<Review> {
        if selections.is_empty() || selections.len() > MAX_ITEMS {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut identities = std::collections::HashSet::new();
        let mut items = Vec::new();
        let mut entries = Vec::new();
        for selection in selections {
            let identity = serde_json::to_string(&selection).map_err(|_| ErrorCode::Internal)?;
            if !identities.insert(identity) {
                continue;
            }
            match &selection {
                Selection::Filesystem {
                    generation,
                    finding_id,
                } => {
                    let details = scanner.inspect(*generation, finding_id)?;
                    let mut item = scanner.with_terminal_scan(*generation, |data| {
                        let finding = data
                            .records
                            .iter()
                            .find(|record| record.finding.id == *finding_id)
                            .ok_or(ErrorCode::InvalidRequest)?;
                        let artifact = data
                            .recognition
                            .findings
                            .iter()
                            .find(|artifact| artifact.finding_id == *finding_id)
                            .cloned();
                        let unavailable_reason =
                            filesystem_unavailability(artifact.as_ref()).into();
                        Ok::<_, ErrorCode>(ReviewItem {
                            selection: selection.clone(),
                            target: details.display_path,
                            logical_bytes: finding.finding.logical_bytes.to_string(),
                            allocated_bytes: Some(finding.finding.allocated_bytes.to_string()),
                            measured_at_ms: Some(finding.finding.measured_at_ms),
                            artifact,
                            unavailable_reason,
                            changed: false,
                            method: None,
                        })
                    })??;
                    // This read-only check detects known changes; it never grants mutation authority.
                    if scanner
                        .reveal_with(*generation, finding_id, |_| Ok(()))
                        .is_err()
                    {
                        item.changed = true;
                        item.unavailable_reason = "Target missing, changed, symlinked or no longer authorized. Rescan before reviewing again.".into();
                    }
                    if reclaim_core::cleanup::filesystem_method(item.artifact.as_ref()).is_ok()
                        && !item.changed
                    {
                        match scanner.cleanup_target(*generation, finding_id) {
                            Ok(target) => {
                                item.method = Some(target.method);
                                item.unavailable_reason = if target.method == Method::Trash {
                                    "Move entire selected directory to macOS Trash, including local edits. Regeneration may require downloads. Recover through macOS Trash / Put Back where available. No app-driven restore.".into()
                                } else {
                                    "Non-force Git linked-worktree removal. Not recoverable through macOS Trash. Uncommitted, untracked and ignored data must be absent.".into()
                                };
                                entries.push(PlanEntry {
                                    selection: selection.clone(),
                                    label: item.target.clone(),
                                    target: PlannedTarget::Filesystem(target),
                                });
                            }
                            Err(_) => {
                                item.changed = true;
                                item.unavailable_reason = "Incomplete, protected, excluded, symlinked or changed target/evidence. Rescan and inspect before a new review.".into();
                            }
                        }
                    }
                    items.push(item);
                }
                Selection::Docker { finding_id } => {
                    if finding_id.len() > 256 {
                        return Err(ErrorCode::InvalidRequest.into());
                    }
                    let (measured_at_ms, item) = docker
                        .observed_item(finding_id)
                        .ok_or(ErrorCode::InvalidRequest)?;
                    let unavailable_reason = if item.persistent {
                        "Containers and volumes may contain persistent data. Removal is prohibited."
                    } else {
                        "Docker removal unavailable: no proven version-specific atomic reference safeguard and pinned local-daemon mutation boundary. Inventory is inspection only."
                    };
                    let prepared = if !item.persistent
                        && item.kind == reclaim_core::docker::ResourceKind::Image
                    {
                        docker.prepare_removal(finding_id).ok()
                    } else {
                        None
                    };
                    let method = prepared.as_ref().map(|_| Method::DockerImage);
                    let unavailable_reason = if method.is_some() {
                        "Remove exact untagged image through Docker without force or parent pruning. Not recoverable through Trash. Concurrent Docker activity may cause partial effects; errors require inspection."
                    } else if item.persistent {
                        unavailable_reason
                    } else {
                        "Inspection only: removal supports Docker 28 classic untagged images with zero known references on a verified local daemon. Version/backend, tags, references or fresh local checks did not qualify."
                    };
                    if let Some(plan) = prepared {
                        entries.push(PlanEntry {
                            selection: selection.clone(),
                            label: item.identity.clone(),
                            target: PlannedTarget::Docker(plan),
                        });
                    }
                    items.push(ReviewItem {
                        selection,
                        target: item.identity,
                        logical_bytes: item.logical_bytes.unwrap_or_else(|| "unknown".into()),
                        allocated_bytes: None,
                        measured_at_ms: Some(measured_at_ms),
                        artifact: Some(item.artifact),
                        unavailable_reason: unavailable_reason.into(),
                        changed: false,
                        method,
                    });
                }
            }
        }
        let (plan_id, expires_at_ms) = self.plan(&mut items, entries)?;
        Ok(Review {
            checked_at_ms: now_ms(),
            items,
            plan_id,
            expires_at_ms,
        })
    }

    pub fn history(&self, directory: &Path) -> HistoryResponse {
        let loaded = {
            let _guard = lock(&self.history_lock);
            crate::cleanup_history::read(directory)
        };
        match loaded {
            Ok(mut history) => {
                if let Some(execution) = self.status().filter(|execution| execution.running) {
                    let _ = history.record(execution.operation);
                }
                HistoryResponse {
                    history: Some(history),
                    unavailable: false,
                }
            }
            Err(_) => HistoryResponse {
                history: None,
                unavailable: true,
            },
        }
    }

    pub fn clear_history(&self, directory: &Path) -> BoundaryResult<()> {
        let state = lock(&self.state);
        if state
            .execution
            .as_ref()
            .is_some_and(|execution| execution.running)
        {
            return Err(ErrorCode::Busy.into());
        }
        let _guard = lock(&self.history_lock);
        crate::cleanup_history::write(directory, &History::default())
    }
}

fn history_directory<R: Runtime>(app: &AppHandle<R>) -> BoundaryResult<PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|_| ErrorCode::Unavailable.into())
}

#[tauri::command]
pub async fn cleanup_review<R: Runtime>(
    window: WebviewWindow<R>,
    cleanup: State<'_, Arc<CleanupService>>,
    scanner: State<'_, Arc<ScanService>>,
    docker: State<'_, Arc<DockerService>>,
    selections: Vec<Selection>,
) -> BoundaryResult<Review> {
    authorize(&window)?;
    let cleanup = cleanup.inner().clone();
    let scanner = scanner.inner().clone();
    let docker = docker.inner().clone();
    tauri::async_runtime::spawn_blocking(move || cleanup.review(&scanner, &docker, selections))
        .await
        .map_err(|_| ErrorCode::Internal)?
}

#[tauri::command]
pub async fn cleanup_history<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
    cleanup: State<'_, Arc<CleanupService>>,
) -> BoundaryResult<HistoryResponse> {
    authorize(&window)?;
    Ok(cleanup.history(&history_directory(&app)?))
}

#[tauri::command]
pub async fn cleanup_clear_history<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
    cleanup: State<'_, Arc<CleanupService>>,
) -> BoundaryResult<()> {
    authorize(&window)?;
    cleanup.clear_history(&history_directory(&app)?)
}

#[tauri::command]
pub async fn cleanup_execute<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
    cleanup: State<'_, Arc<CleanupService>>,
    scanner: State<'_, Arc<ScanService>>,
    docker: State<'_, Arc<DockerService>>,
    plan_id: String,
    confirmed: bool,
) -> BoundaryResult<Execution> {
    authorize(&window)?;
    let directory = history_directory(&app)?;
    let cleanup = cleanup.inner().clone();
    let scanner = scanner.inner().clone();
    let docker = docker.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        cleanup.execute(&scanner, &docker, &directory, &plan_id, confirmed)
    })
    .await
    .map_err(|_| ErrorCode::Internal)?
}

#[tauri::command]
pub async fn cleanup_status<R: Runtime>(
    window: WebviewWindow<R>,
    cleanup: State<'_, Arc<CleanupService>>,
) -> BoundaryResult<Option<Execution>> {
    authorize(&window)?;
    Ok(cleanup.status())
}

#[tauri::command]
pub async fn cleanup_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    cleanup: State<'_, Arc<CleanupService>>,
) -> BoundaryResult<()> {
    authorize(&window)?;
    cleanup.cancel();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};

    #[test]
    fn cleanup_ipc_rejects_paths_commands_forged_ids_and_other_windows() {
        let mut context = crate::test_context();
        context.config_mut().app.windows.clear();
        let app = mock_builder()
            .manage(Arc::new(CleanupService::default()))
            .manage(Arc::new(ScanService::default()))
            .manage(Arc::new(DockerService::default()))
            .invoke_handler(tauri::generate_handler![
                cleanup_review,
                cleanup_history,
                cleanup_clear_history
            ])
            .build(context)
            .unwrap();
        for label in ["main", "other"] {
            let window = tauri::WebviewWindowBuilder::new(&app, label, Default::default())
                .build()
                .unwrap();
            for body in [
                json!({"selections":[{"source":"filesystem","generation":1,"findingId":"/outside","path":"/outside","force":true}]}),
                json!({"selections":[{"source":"docker","findingId":"forged"}]}),
                json!({"selections":[{"source":"shell","command":"rm"}]}),
            ] {
                let response = get_ipc_response(
                    &window,
                    tauri::webview::InvokeRequest {
                        cmd: "cleanup_review".into(),
                        callback: tauri::ipc::CallbackFn(0),
                        error: tauri::ipc::CallbackFn(1),
                        url: window.url().unwrap(),
                        body: tauri::ipc::InvokeBody::Json(body),
                        headers: Default::default(),
                        invoke_key: INVOKE_KEY.into(),
                    },
                );
                assert!(response.is_err());
            }
            if label == "other" {
                for command in ["cleanup_history", "cleanup_clear_history"] {
                    let response = get_ipc_response(
                        &window,
                        tauri::webview::InvokeRequest {
                            cmd: command.into(),
                            callback: tauri::ipc::CallbackFn(0),
                            error: tauri::ipc::CallbackFn(1),
                            url: window.url().unwrap(),
                            body: tauri::ipc::InvokeBody::Json(json!({})),
                            headers: Default::default(),
                            invoke_key: INVOKE_KEY.into(),
                        },
                    );
                    assert!(response.is_err());
                }
            }
        }
    }
    mod isolated_fixture {
        use super::{
            authorize, execution, lock, Arc, BoundaryResult, CleanupService, Execution, Mutex,
            PathBuf, PlannedTarget, ScanService, State, WebviewWindow,
        };
        pub(super) struct FixtureStore {
            pub(super) directory: PathBuf,
            pub(super) restored: Mutex<Vec<(PathBuf, PathBuf)>>,
        }
        #[tauri::command(rename = "cleanup_execute")]
        pub(super) fn cleanup_fixture_execute(
            window: WebviewWindow<tauri::test::MockRuntime>,
            fixture: State<'_, Arc<FixtureStore>>,
            cleanup: State<'_, Arc<CleanupService>>,
            scanner: State<'_, Arc<ScanService>>,
            plan_id: String,
            confirmed: bool,
        ) -> BoundaryResult<Execution> {
            authorize(&window)?;
            cleanup.execute_with(
                &scanner,
                &fixture.directory,
                &plan_id,
                confirmed,
                |target, _control| {
                    let PlannedTarget::Filesystem(target) = target else {
                        panic!("filesystem fixture only")
                    };
                    let moved =
                        execution::trash(&target.path).expect("actual platform fixture Trash");
                    lock(&fixture.restored).push((target.path.clone(), moved));
                    (
                    reclaim_core::cleanup::Outcome::Removed,
                    "Moved exact disposable fixture to macOS Trash. Recovery through macOS Trash."
                        .into(),
                )
                },
            )
        }
    }
    use isolated_fixture::FixtureStore;
    #[test]
    fn native_ipc_plan_confirmation_actual_fixture_trash_and_restart_history() {
        let (temporary, scanner, generation) = crate::scan::cleanup_review_tests::fixture();
        let finding =
            crate::scan::cleanup_review_tests::select(&scanner, generation, "node_modules");
        let history = tempfile::tempdir().unwrap();
        let store = Arc::new(FixtureStore {
            directory: history.path().canonicalize().unwrap(),
            restored: Mutex::new(Vec::new()),
        });
        let cleanup = Arc::new(CleanupService::default());
        let mut context = crate::test_context();
        context.config_mut().app.windows.clear();
        let app = mock_builder()
            .manage(cleanup.clone())
            .manage(scanner)
            .manage(Arc::new(DockerService::default()))
            .manage(store.clone())
            .invoke_handler(tauri::generate_handler![
                cleanup_review,
                isolated_fixture::cleanup_fixture_execute,
                cleanup_status,
                cleanup_cancel
            ])
            .build(context)
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let call = |command: &str, body: serde_json::Value| {
            get_ipc_response(
                &window,
                tauri::webview::InvokeRequest {
                    cmd: command.into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: window.url().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(body),
                    headers: Default::default(),
                    invoke_key: INVOKE_KEY.into(),
                },
            )
            .map(|response| response.deserialize::<serde_json::Value>().unwrap())
        };
        let reviewed = call("cleanup_review", json!({"selections":[finding]})).unwrap();
        assert_eq!(reviewed["items"][0]["method"], "trash");
        let plan = reviewed["planId"].as_str().unwrap();
        assert!(call("cleanup_execute", json!({"planId":plan,"confirmed":false})).is_err());
        assert!(temporary.path().join("node_modules").exists());
        let result = call("cleanup_execute", json!({"planId":plan,"confirmed":true})).unwrap();
        assert_eq!(result["operation"]["items"][0]["outcome"], "removed");
        assert!(!temporary.path().join("node_modules").exists());
        assert!(call("cleanup_execute", json!({"planId":plan,"confirmed":true})).is_err());
        assert_eq!(
            CleanupService::default()
                .history(&store.directory)
                .history
                .unwrap()
                .operations[0]
                .items[0]
                .outcome,
            reclaim_core::cleanup::Outcome::Removed
        );
        assert_eq!(
            std::fs::read(temporary.path().join("source.rs")).unwrap(),
            b"source sentinel"
        );
        assert_eq!(
            std::fs::read(temporary.path().join(".env")).unwrap(),
            b"secret sentinel"
        );
        for (original, moved) in lock(&store.restored).drain(..) {
            assert!(!original.exists());
            std::fs::rename(moved, original).unwrap();
        }
    }
}
