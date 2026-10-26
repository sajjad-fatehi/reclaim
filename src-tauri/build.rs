fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "scan_locations",
            "scan_pick_folder",
            "scan_register_scope",
            "scan_start",
            "scan_current",
            "scan_status",
            "scan_page",
            "scan_cancel",
            "recognition_page",
        ]),
    ))
    .expect("Failed to build native application permissions");
}
