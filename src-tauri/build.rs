fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "preferences_load",
            "preferences_save",
            "scan_locations",
            "scan_pick_folder",
            "scan_register_scope",
            "scan_start",
            "scan_current",
            "scan_status",
            "scan_page",
            "scan_cancel",
            "recognition_page",
            "scan_inspect",
            "scan_reveal",
            "docker_inventory",
            "docker_cancel",
            "cleanup_review",
            "cleanup_history",
            "cleanup_clear_history",
        ]),
    ))
    .expect("Failed to build native application permissions");
}
