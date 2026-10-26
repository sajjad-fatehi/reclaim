//! Interactive production React/IPC QA over isolated synthetic folders/configuration.
use reclaim_desktop::{
    preferences::{PreferenceUpdate, Preferences, PreferencesService},
    scan::{self, BoundaryResult, Location, Locations, ScanService},
};
use std::sync::Arc;
use tauri::{State, WebviewWindow};

struct Fixture {
    configuration: std::path::PathBuf,
    candidates: Vec<Location>,
}

#[tauri::command]
async fn scan_locations(fixture: State<'_, Fixture>) -> BoundaryResult<Locations> {
    Ok(Locations {
        candidates: fixture.candidates.clone(),
        unavailable_count: 0,
    })
}

#[tauri::command]
async fn preferences_load(
    fixture: State<'_, Fixture>,
    scanner: State<'_, Arc<ScanService>>,
    preferences: State<'_, PreferencesService>,
) -> BoundaryResult<Preferences> {
    Ok(preferences.load(&fixture.configuration, &scanner))
}

#[tauri::command]
async fn preferences_save(
    fixture: State<'_, Fixture>,
    scanner: State<'_, Arc<ScanService>>,
    preferences: State<'_, PreferencesService>,
    update: PreferenceUpdate,
) -> BoundaryResult<()> {
    preferences.save(&fixture.configuration, &scanner, update)
}

#[tauri::command]
async fn scan_pick_folder(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<'_, Arc<ScanService>>,
) -> BoundaryResult<Option<Location>> {
    scan::scan_pick_folder(window, app, state).await
}

fn main() {
    let temporary = tempfile::Builder::new()
        .prefix("reclaim-SYNTHETIC-overview-")
        .tempdir()
        .unwrap();
    let selected = temporary.path().join("SYNTHETIC-small");
    let large = temporary.path().join("SYNTHETIC-large");
    std::fs::create_dir_all(selected.join("cache")).unwrap();
    std::fs::create_dir(&large).unwrap();
    std::fs::write(selected.join("synthetic.txt"), b"12345").unwrap();
    std::fs::write(selected.join("synthetic.jpg"), vec![0u8; 32768]).unwrap();
    std::fs::write(selected.join("cache/excluded.bin"), vec![0u8; 1024]).unwrap();
    for index in 0..20_000 {
        let directory = large.join(format!("synthetic-{index}"));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("synthetic.txt"), b"x").unwrap();
    }
    let scanner = Arc::new(ScanService::default());
    let candidates = vec![
        scanner.register_native_location(&selected).unwrap(),
        scanner.register_native_location(&large).unwrap(),
    ];
    println!("SYNTHETIC fixture picker path: {}", selected.display());
    let builder = tauri::Builder::default()
        .manage(Fixture {
            configuration: temporary.path().join("preferences.json"),
            candidates,
        })
        .manage(scanner)
        .manage(PreferencesService::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_locations,
            preferences_load,
            preferences_save,
            scan_pick_folder,
            scan::scan_register_scope,
            scan::scan_start,
            scan::scan_current,
            scan::scan_status,
            scan::scan_page,
            scan::scan_cancel
        ]);
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    context.config_mut().build.dev_url = None;
    let app = builder.build(context).unwrap();
    tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::App("index.html".into()))
        .initialization_script("window.addEventListener('error', event => { location.href = '/fixture-error?message=' + encodeURIComponent(event.message); });")
        .on_navigation(|url| {
            if url.path() == "/fixture-error" {
                eprintln!("Synthetic fixture render error: {}", url.query().unwrap_or_default());
                return false;
            }
            true
        })
        .title("Reclaim — SYNTHETIC Overview fixture")
        .inner_size(1200.0, 900.0)
        .build()
        .unwrap();
    let exit = app.run_return(|_, _| {});
    drop(temporary);
    std::process::exit(exit);
}
