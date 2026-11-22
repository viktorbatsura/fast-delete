pub mod engine;

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemInfo {
    app_version: &'static str,
    logical_cpus: usize,
}

#[tauri::command]
fn system_info() -> SystemInfo {
    let logical_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    SystemInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        logical_cpus,
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![system_info])
        .run(tauri::generate_context!())
        .expect("failed to start Fast Delete");
}
