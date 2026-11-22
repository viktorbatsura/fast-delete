mod commands;
pub mod drives;
pub mod engine;
pub mod nav;
pub mod queue;

#[cfg(test)]
mod test_support;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            commands::init(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system_info,
            commands::list_drives,
            commands::list_dir,
            commands::suggest_paths,
            commands::queue_snapshot,
            commands::queue_add,
            commands::queue_remove,
            commands::queue_set_keep_root,
            commands::queue_clear_finished,
            commands::queue_start,
            commands::queue_cancel,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Fast Delete");
}
