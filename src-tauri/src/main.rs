#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use reclaim_desktop::{preferences, scan};

fn main() {
    tauri::Builder::default()
        .manage(std::sync::Arc::new(scan::ScanService::default()))
        .manage(preferences::PreferencesService::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            preferences::preferences_load,
            preferences::preferences_save,
            scan::scan_locations,
            scan::scan_pick_folder,
            scan::scan_register_scope,
            scan::scan_start,
            scan::scan_current,
            scan::scan_status,
            scan::scan_page,
            scan::scan_cancel,
            scan::recognition_page
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Reclaim desktop application");
}
