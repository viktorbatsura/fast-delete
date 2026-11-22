//! Shared state of one running job: lock-free counters, a cancel flag and a capped error list.
//!
//! Worker threads write to it, the progress reporter and the queue read from it at the same time.

use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use serde::Serialize;

/// Only the first errors are kept with details; the counter still counts all of them.
pub const MAX_STORED_ERRORS: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressSnapshot {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub errors: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug)]
pub struct Progress {
    files: AtomicU64,
    dirs: AtomicU64,
    bytes: AtomicU64,
    errors: AtomicU64,
    cancelled: AtomicBool,
    started: Instant,
    error_log: Mutex<Vec<DeleteError>>,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            files: AtomicU64::new(0),
            dirs: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            cancelled: AtomicBool::new(false),
            started: Instant::now(),
            error_log: Mutex::new(Vec::new()),
        }
    }

    // Relaxed ordering is enough: counters are independent statistics,
    // no other data is published through them.
    pub fn add_file(&self, size: u64) {
        self.files.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(size, Ordering::Relaxed);
    }

    pub fn add_dir(&self) {
        self.dirs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self, path: &Path, error: impl fmt::Display) {
        self.errors.fetch_add(1, Ordering::Relaxed);

        // A panic in another thread must not break error reporting, so a poisoned lock is reused.
        let mut log = self.error_log.lock().unwrap_or_else(PoisonError::into_inner);
        if log.len() < MAX_STORED_ERRORS {
            log.push(DeleteError {
                path: path.to_string_lossy().into_owned(),
                message: error.to_string(),
            });
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            files: self.files.load(Ordering::Relaxed),
            dirs: self.dirs.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            elapsed_ms: u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
        }
    }

    /// Moves the stored errors out, leaving the list empty.
    pub fn take_errors(&self) -> Vec<DeleteError> {
        let mut log = self.error_log.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *log)
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn progress_can_be_shared_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Progress>();
    }

    #[test]
    fn counters_add_up_across_threads() {
        let progress = Progress::new();

        // Scoped threads may borrow `progress` because they are joined before the scope ends.
        thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for _ in 0..1000 {
                        progress.add_file(10);
                    }
                    progress.add_dir();
                });
            }
        });

        let snap = progress.snapshot();
        assert_eq!(snap.files, 8000);
        assert_eq!(snap.bytes, 80_000);
        assert_eq!(snap.dirs, 8);
        assert_eq!(snap.errors, 0);
    }

    #[test]
    fn cancel_is_visible_from_another_thread() {
        let progress = Arc::new(Progress::new());
        let worker_view = Arc::clone(&progress);

        let worker = thread::spawn(move || {
            while !worker_view.is_cancelled() {
                thread::yield_now();
            }
        });

        progress.cancel();
        worker.join().unwrap();
        assert!(progress.is_cancelled());
    }

    #[test]
    fn errors_are_counted_but_stored_with_a_cap() {
        let progress = Progress::new();
        for i in 0..MAX_STORED_ERRORS + 5 {
            progress.record_error(Path::new(&format!(r"C:\tmp\{i}")), "Access is denied");
        }

        assert_eq!(progress.snapshot().errors, (MAX_STORED_ERRORS + 5) as u64);

        let stored = progress.take_errors();
        assert_eq!(stored.len(), MAX_STORED_ERRORS);
        assert_eq!(stored[0].message, "Access is denied");
        assert!(progress.take_errors().is_empty());
    }
}
