//! Full Disk Access guidance. Detection is a read-only probe and the opener targets one fixed System Settings pane.
use crate::scan::{authorize, BoundaryResult, ErrorCode};
use reclaim_core::access::{self, FullDiskAccess};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

const FULL_DISK_ACCESS_PANE: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessStatus {
    pub full_disk_access: FullDiskAccess,
}

#[tauri::command]
pub async fn access_status<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
) -> BoundaryResult<AccessStatus> {
    authorize(&window)?;
    let home = app.path().home_dir().ok();
    tauri::async_runtime::spawn_blocking(move || AccessStatus {
        full_disk_access: access::detect(home.as_deref()),
    })
    .await
    .map_err(|_| ErrorCode::Internal.into())
}

#[tauri::command]
pub async fn access_open_settings<R: Runtime>(window: WebviewWindow<R>) -> BoundaryResult<()> {
    authorize(&window)?;
    open_full_disk_access_pane()
}

fn open_full_disk_access_pane() -> BoundaryResult<()> {
    let address = objc2_foundation::NSString::from_str(FULL_DISK_ACCESS_PANE);
    let url = objc2_foundation::NSURL::URLWithString(&address).ok_or(ErrorCode::Unavailable)?;
    if objc2_app_kit::NSWorkspace::sharedWorkspace().openURL(&url) {
        return Ok(());
    }
    Err(ErrorCode::Unavailable.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opener_target_is_the_fixed_full_disk_access_pane() {
        assert_eq!(
            FULL_DISK_ACCESS_PANE,
            "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
        );
    }

    #[test]
    fn status_serializes_with_stable_snake_case_values() {
        for (status, expected) in [
            (FullDiskAccess::Granted, "granted"),
            (FullDiskAccess::NotGranted, "not_granted"),
            (FullDiskAccess::Unknown, "unknown"),
        ] {
            let json = serde_json::to_value(AccessStatus {
                full_disk_access: status,
            })
            .unwrap();
            assert_eq!(json["fullDiskAccess"], expected);
        }
    }
}
