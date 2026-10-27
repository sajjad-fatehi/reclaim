//! Read-only cleanup eligibility. No mutation endpoint exists until its invariants are proven.
use crate::docker::DockerService;
use crate::scan::{authorize, BoundaryResult, ErrorCode, ScanService};
use reclaim_core::cleanup::{filesystem_unavailability, History, MAX_ITEMS};
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub checked_at_ms: u64,
    pub items: Vec<ReviewItem>,
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
                    items.push(ReviewItem {
                        selection,
                        target: item.identity,
                        logical_bytes: item.logical_bytes.unwrap_or_else(|| "unknown".into()),
                        allocated_bytes: None,
                        measured_at_ms: Some(measured_at_ms),
                        artifact: Some(item.artifact),
                        unavailable_reason: unavailable_reason.into(),
                        changed: false,
                    });
                }
            }
        }
        Ok(Review {
            checked_at_ms: now_ms(),
            items,
        })
    }

    pub fn history(&self, directory: &Path) -> HistoryResponse {
        let _guard = lock(&self.history_lock);
        match crate::cleanup_history::read(directory) {
            Ok(history) => HistoryResponse {
                history: Some(history),
                unavailable: false,
            },
            Err(_) => HistoryResponse {
                history: None,
                unavailable: true,
            },
        }
    }

    pub fn clear_history(&self, directory: &Path) -> BoundaryResult<()> {
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
    cleanup: State<'_, CleanupService>,
    scanner: State<'_, Arc<ScanService>>,
    docker: State<'_, Arc<DockerService>>,
    selections: Vec<Selection>,
) -> BoundaryResult<Review> {
    authorize(&window)?;
    cleanup.review(&scanner, &docker, selections)
}

#[tauri::command]
pub async fn cleanup_history<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
    cleanup: State<'_, CleanupService>,
) -> BoundaryResult<HistoryResponse> {
    authorize(&window)?;
    Ok(cleanup.history(&history_directory(&app)?))
}

#[tauri::command]
pub async fn cleanup_clear_history<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
    cleanup: State<'_, CleanupService>,
) -> BoundaryResult<()> {
    authorize(&window)?;
    cleanup.clear_history(&history_directory(&app)?)
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
            .manage(CleanupService::default())
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
}
