//! Disposable native WebView/IPC proof. Never scans user locations.
use reclaim_desktop::scan::{self, ScanService};
use std::sync::Arc;

fn main() {
    let fixture = tempfile::tempdir().expect("create disposable synthetic fixture");
    let small = fixture.path().join("small");
    let large = fixture.path().join("large");
    std::fs::create_dir(&small).unwrap();
    std::fs::create_dir(&large).unwrap();
    std::fs::write(small.join("synthetic.txt"), b"12345").unwrap();
    for index in 0..20_000 {
        std::fs::write(large.join(format!("synthetic-{index}")), b"x").unwrap();
    }
    let service = Arc::new(ScanService::default());
    let small_id = service.register_native_location(&small).unwrap().id;
    let large_id = service.register_native_location(&large).unwrap().id;
    let script = include_str!("scan_smoke.js")
        .replace("SMALL_LOCATION_ID", &small_id)
        .replace("LARGE_LOCATION_ID", &large_id);
    let keep_open = std::env::var_os("RECLAIM_SMOKE_KEEP_OPEN").is_some();
    let builder = tauri::Builder::default()
        .manage(service)
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan::scan_locations,
            scan::scan_pick_folder,
            scan::scan_register_scope,
            scan::scan_start,
            scan::scan_current,
            scan::scan_status,
            scan::scan_page,
            scan::scan_cancel
        ])
        .on_page_load(move |window, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                window.eval(&script).expect("run fixture IPC script");
            }
        });
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app = builder.build(context).expect("native fixture app");
    let handle = app.handle().clone();
    // A local navigation reports completion without adding a test-only IPC command.
    tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::App("index.html".into()))
        .title("Reclaim — SYNTHETIC scan IPC fixture")
        .inner_size(720.0, 520.0)
        .on_navigation(move |url| {
            if url.path() == "/scan-smoke-pass" {
                println!(
                    "SYNTHETIC native WebView IPC PASS: {}",
                    url.query().unwrap_or_default()
                );
                if !keep_open {
                    handle.exit(0);
                }
                return false;
            }
            if url.path() == "/scan-smoke-fail" {
                eprintln!(
                    "SYNTHETIC native WebView IPC FAIL: {}",
                    url.query().unwrap_or_default()
                );
                handle.exit(1);
                return false;
            }
            true
        })
        .build()
        .unwrap();
    let exit_code = app.run_return(|_, _| {});
    drop(fixture);
    std::process::exit(exit_code);
}
