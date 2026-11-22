//! Thin Tauri layer: turns UI calls into core calls and core notifications into UI events.
//!
//! No business logic lives here. Errors are converted to strings because that is what the UI shows.

use std::path::Path;
use std::sync::Arc;

use serde::Serialize;
use tauri::{App, AppHandle, Emitter, Manager, State};

use crate::drives::{self, DriveInfo};
use crate::engine::deleter::{DeleteOptions, Mode};
use crate::engine::progress::ProgressSnapshot;
use crate::engine::safety::SafetyRules;
use crate::nav::{self, DirListing};
use crate::queue::{AddOutcome, JobQueue, QueueEvents, QueueSnapshot};

pub const EVENT_QUEUE_CHANGED: &str = "queue-changed";
pub const EVENT_JOB_PROGRESS: &str = "job-progress";

/// Creates the queue with Tauri as its event listener and registers it as managed state.
pub fn init(app: &mut App) {
    let events = Arc::new(TauriEvents {
        app: app.handle().clone(),
    });
    app.manage(JobQueue::new(SafetyRules::from_environment(), events));
}

struct TauriEvents {
    app: AppHandle,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobProgressPayload {
    item_id: u64,
    progress: ProgressSnapshot,
}

// A failed emit only means the window is gone; the job itself must keep going.
impl QueueEvents for TauriEvents {
    fn queue_changed(&self, snapshot: &QueueSnapshot) {
        let _ = self.app.emit(EVENT_QUEUE_CHANGED, snapshot);
    }

    fn job_progress(&self, item_id: u64, progress: &ProgressSnapshot) {
        let payload = JobProgressPayload {
            item_id,
            progress: *progress,
        };
        let _ = self.app.emit(EVENT_JOB_PROGRESS, payload);
    }
}

/// Runs blocking file-system work off the main thread so the window never freezes.
async fn blocking<T, F>(work: F) -> Result<T, String>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    app_version: &'static str,
    logical_cpus: usize,
    default_threads: usize,
}

#[tauri::command]
pub fn system_info() -> SystemInfo {
    SystemInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        logical_cpus: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        default_threads: DeleteOptions::default_threads(),
    }
}

#[tauri::command]
pub async fn list_drives() -> Result<Vec<DriveInfo>, String> {
    blocking(drives::list_drives).await
}

#[tauri::command]
pub async fn list_dir(path: String) -> Result<DirListing, String> {
    blocking(move || nav::list_dir(Path::new(&path)).map_err(|e| e.to_string())).await?
}

#[tauri::command]
pub async fn suggest_paths(input: String) -> Result<Vec<String>, String> {
    blocking(move || nav::suggest(&input)).await
}

#[tauri::command]
pub fn queue_snapshot(queue: State<'_, JobQueue>) -> QueueSnapshot {
    queue.snapshot()
}

/// Async because validating paths touches the disk (a slow network path must not freeze the UI).
#[tauri::command]
pub async fn queue_add(queue: State<'_, JobQueue>, paths: Vec<String>) -> Result<AddOutcome, String> {
    let queue = queue.inner().clone();
    blocking(move || queue.add(&paths)).await
}

#[tauri::command]
pub fn queue_remove(queue: State<'_, JobQueue>, id: u64) -> Result<(), String> {
    queue.remove(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn queue_set_keep_root(queue: State<'_, JobQueue>, id: u64, keep_root: bool) -> Result<(), String> {
    queue.set_keep_root(id, keep_root).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn queue_clear_finished(queue: State<'_, JobQueue>) {
    queue.clear_finished();
}

#[tauri::command]
pub fn queue_start(queue: State<'_, JobQueue>, mode: Mode, threads: Option<usize>) -> Result<(), String> {
    queue.start(mode, threads).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn queue_cancel(queue: State<'_, JobQueue>) {
    queue.cancel();
}
