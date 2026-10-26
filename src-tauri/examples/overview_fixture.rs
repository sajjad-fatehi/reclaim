//! Interactive production React/IPC QA over isolated synthetic folders/configuration.
use reclaim_desktop::{
    preferences::{PreferenceUpdate, Preferences, PreferencesService},
    scan::{self, BoundaryResult, Location, Locations, ScanService},
};
use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow};

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
    let compact = std::env::args().any(|argument| argument == "--compact");
    let enlarged = std::env::args().any(|argument| argument == "--enlarged");
    let selected = temporary.path().join("SYNTHETIC-small");
    let large = temporary.path().join("SYNTHETIC-large");
    std::fs::create_dir_all(selected.join("cache")).unwrap();
    std::fs::create_dir(&large).unwrap();
    std::fs::write(selected.join("synthetic.txt"), b"12345").unwrap();
    std::fs::create_dir_all(selected.join("開発/node_modules")).unwrap();
    std::fs::create_dir(selected.join("other")).unwrap();
    std::fs::write(
        selected.join("開発/package.json"),
        r#"{"name":"SYNTHETIC-project"}"#,
    )
    .unwrap();
    std::fs::write(
        selected.join("開発/node_modules/.package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{}}"#,
    )
    .unwrap();
    std::fs::write(selected.join("開発/.hidden"), b"fixture").unwrap();
    std::fs::write(selected.join("開発/same.txt"), b"fixture").unwrap();
    std::fs::write(selected.join("other/same.txt"), b"fixture").unwrap();
    for index in 0..125 {
        std::fs::write(
            selected
                .join("other")
                .join(format!("SYNTHETIC-{index}.txt")),
            b"fixture",
        )
        .unwrap();
    }
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
            scan::scan_cancel,
            scan::explore::scan_inspect,
            scan::explore::scan_reveal
        ]);
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    context.config_mut().build.dev_url = None;
    let app = builder.build(context).unwrap();
    tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::App("index.html".into()))
        .initialization_script(if enlarged { "document.addEventListener('DOMContentLoaded', () => { document.documentElement.style.fontSize = '200%'; });" } else { "" })
        .initialization_script("window.addEventListener('error', event => { location.href = '/fixture-error?message=' + encodeURIComponent(event.message); });")
        .on_navigation(|url| {
            if url.path() == "/fixture-error" {
                eprintln!("Synthetic fixture render error: {}", url.query().unwrap_or_default());
                return false;
            }
            true
        })
        .title("Reclaim — SYNTHETIC Explore fixture")
        .inner_size(if compact { 900.0 } else { 1440.0 }, if compact { 600.0 } else { 1000.0 })
        .build()
        .unwrap();
    let exit = app.run_return(|app, event| {
        if matches!(event, tauri::RunEvent::Ready) {
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            window.set_focus().unwrap();
            println!(
                "SYNTHETIC window visible={} size={:?} position={:?}",
                window.is_visible().unwrap(),
                window.inner_size().unwrap(),
                window.outer_position().unwrap()
            );
        }
    });
    drop(temporary);
    std::process::exit(exit);
}
