//! Job queue: the single source of truth for what is queued, running and finished.
//!
//! Jobs run one at a time on a background worker thread. The queue knows nothing about Tauri:
//! it reports changes through the `QueueEvents` trait, so it can be tested with a fake listener.

use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use serde::Serialize;

use crate::engine::deleter::{self, DeleteOptions, Mode, Outcome};
use crate::engine::progress::{DeleteError, Progress, ProgressSnapshot};
use crate::engine::safety::SafetyRules;

const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

/// Receives queue notifications. Implemented by the Tauri layer (events to the UI) and by tests.
pub trait QueueEvents: Send + Sync {
    fn queue_changed(&self, snapshot: &QueueSnapshot);
    fn job_progress(&self, item_id: u64, progress: &ProgressSnapshot);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    Idle,
    Queued,
    Running,
    Completed,
    CompletedWithErrors,
    Cancelled,
    Failed,
}

impl From<Outcome> for ItemStatus {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Completed => Self::Completed,
            Outcome::CompletedWithErrors => Self::CompletedWithErrors,
            Outcome::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub id: u64,
    pub path: String,
    pub keep_root: bool,
    pub status: ItemStatus,
    /// Mode of the current or last run.
    pub mode: Option<Mode>,
    /// Totals of the last successful scan, used as an estimate while deleting.
    pub estimate: Option<ProgressSnapshot>,
    /// Final counters of the last run.
    pub result: Option<ProgressSnapshot>,
    pub errors: Vec<DeleteError>,
    pub message: Option<String>,
}

impl QueueItem {
    fn is_runnable(&self) -> bool {
        match self.status {
            ItemStatus::Idle
            | ItemStatus::Cancelled
            | ItemStatus::Failed
            | ItemStatus::CompletedWithErrors => true,
            // A finished scan can be followed by a deletion; a finished deletion has nothing left.
            ItemStatus::Completed => self.mode == Some(Mode::Scan),
            ItemStatus::Queued | ItemStatus::Running => false,
        }
    }

    fn is_finished(&self) -> bool {
        matches!(
            self.status,
            ItemStatus::Completed | ItemStatus::CompletedWithErrors | ItemStatus::Cancelled | ItemStatus::Failed
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueSnapshot {
    pub items: Vec<QueueItem>,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rejection {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddOutcome {
    pub added: Vec<u64>,
    pub rejected: Vec<Rejection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    AlreadyRunning,
    NothingToRun,
    NotFound(u64),
    Busy(u64),
    Worker(String),
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "a job is already running"),
            Self::NothingToRun => write!(f, "nothing to run: add folders first"),
            Self::NotFound(id) => write!(f, "queue item {id} not found"),
            Self::Busy(id) => write!(f, "queue item {id} is queued or running"),
            Self::Worker(msg) => write!(f, "cannot start worker thread: {msg}"),
        }
    }
}

impl std::error::Error for QueueError {}

pub struct JobQueue {
    shared: Arc<Shared>,
}

struct Shared {
    state: Mutex<State>,
    events: Arc<dyn QueueEvents>,
    rules: SafetyRules,
}

#[derive(Default)]
struct State {
    items: Vec<QueueItem>,
    next_id: u64,
    running: bool,
    /// Progress of the running job, kept here so `cancel` can reach it.
    current: Option<Arc<Progress>>,
}

struct Job {
    id: u64,
    path: PathBuf,
    keep_root: bool,
    progress: Arc<Progress>,
}

struct Finished {
    status: ItemStatus,
    counters: ProgressSnapshot,
    errors: Vec<DeleteError>,
    message: Option<String>,
}

impl JobQueue {
    pub fn new(rules: SafetyRules, events: Arc<dyn QueueEvents>) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State::default()),
                events,
                rules,
            }),
        }
    }

    pub fn snapshot(&self) -> QueueSnapshot {
        self.shared.lock().snapshot()
    }

    pub fn add(&self, paths: &[String]) -> AddOutcome {
        let mut outcome = AddOutcome::default();
        {
            let mut state = self.shared.lock();
            for raw in paths {
                match self.shared.rules.check_deletable(std::path::Path::new(raw)) {
                    Err(e) => outcome.rejected.push(Rejection {
                        path: raw.clone(),
                        reason: e.to_string(),
                    }),
                    Ok(path) => {
                        let path = path.to_string_lossy().into_owned();
                        if state.contains(&path) {
                            outcome.rejected.push(Rejection {
                                path,
                                reason: "already in the queue".into(),
                            });
                        } else {
                            outcome.added.push(state.push(path));
                        }
                    }
                }
            }
        }
        if !outcome.added.is_empty() {
            self.shared.emit_snapshot();
        }
        outcome
    }

    pub fn remove(&self, id: u64) -> Result<(), QueueError> {
        {
            let mut state = self.shared.lock();
            let index = state.index_of(id)?;
            if matches!(state.items[index].status, ItemStatus::Queued | ItemStatus::Running) {
                return Err(QueueError::Busy(id));
            }
            state.items.remove(index);
        }
        self.shared.emit_snapshot();
        Ok(())
    }

    pub fn set_keep_root(&self, id: u64, keep_root: bool) -> Result<(), QueueError> {
        {
            let mut state = self.shared.lock();
            let index = state.index_of(id)?;
            let item = &mut state.items[index];
            if matches!(item.status, ItemStatus::Queued | ItemStatus::Running) {
                return Err(QueueError::Busy(id));
            }
            item.keep_root = keep_root;
        }
        self.shared.emit_snapshot();
        Ok(())
    }

    pub fn clear_finished(&self) {
        self.shared.lock().items.retain(|item| !item.is_finished());
        self.shared.emit_snapshot();
    }

    /// Queues every runnable item and starts a background worker that processes them one by one.
    pub fn start(&self, mode: Mode, threads: Option<usize>) -> Result<(), QueueError> {
        let threads = threads
            .unwrap_or_else(DeleteOptions::default_threads)
            .clamp(1, 64);

        {
            let mut state = self.shared.lock();
            if state.running {
                return Err(QueueError::AlreadyRunning);
            }
            let mut queued = 0;
            for item in state.items.iter_mut().filter(|item| item.is_runnable()) {
                item.status = ItemStatus::Queued;
                item.mode = Some(mode);
                queued += 1;
            }
            if queued == 0 {
                return Err(QueueError::NothingToRun);
            }
            state.running = true;
        }
        self.shared.emit_snapshot();

        let shared = Arc::clone(&self.shared);
        let spawned = thread::Builder::new()
            .name("fd-queue".into())
            .spawn(move || worker_loop(&shared, mode, threads));

        if let Err(e) = spawned {
            self.shared.lock().reset_queued();
            self.shared.emit_snapshot();
            return Err(QueueError::Worker(e.to_string()));
        }
        Ok(())
    }

    /// Stops the running job and returns all waiting items to idle.
    pub fn cancel(&self) {
        {
            let mut state = self.shared.lock();
            state.reset_queued();
            if let Some(progress) = &state.current {
                progress.cancel();
            }
        }
        self.shared.emit_snapshot();
    }
}

impl Shared {
    // A panic elsewhere must not lock the queue forever, so a poisoned lock is reused.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The snapshot is taken under the lock, but the event is sent after the lock is released.
    fn emit_snapshot(&self) {
        let snapshot = self.lock().snapshot();
        self.events.queue_changed(&snapshot);
    }
}

impl State {
    fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            items: self.items.clone(),
            running: self.running,
        }
    }

    /// Windows paths are case-insensitive.
    fn contains(&self, path: &str) -> bool {
        let path = path.to_lowercase();
        self.items.iter().any(|item| item.path.to_lowercase() == path)
    }

    fn push(&mut self, path: String) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.items.push(QueueItem {
            id,
            path,
            keep_root: false,
            status: ItemStatus::Idle,
            mode: None,
            estimate: None,
            result: None,
            errors: Vec::new(),
            message: None,
        });
        id
    }

    fn index_of(&self, id: u64) -> Result<usize, QueueError> {
        self.items
            .iter()
            .position(|item| item.id == id)
            .ok_or(QueueError::NotFound(id))
    }

    fn reset_queued(&mut self) {
        for item in self.items.iter_mut().filter(|item| item.status == ItemStatus::Queued) {
            item.status = ItemStatus::Idle;
        }
    }

    fn start_next(&mut self) -> Option<Job> {
        let item = self
            .items
            .iter_mut()
            .find(|item| item.status == ItemStatus::Queued)?;

        item.status = ItemStatus::Running;
        item.result = None;
        item.errors.clear();
        item.message = None;

        let job = Job {
            id: item.id,
            path: PathBuf::from(&item.path),
            keep_root: item.keep_root,
            progress: Arc::new(Progress::new()),
        };
        self.current = Some(Arc::clone(&job.progress));
        Some(job)
    }

    fn finish(&mut self, id: u64, mode: Mode, finished: Finished) {
        self.current = None;
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else {
            return;
        };
        if mode == Mode::Scan && finished.status == ItemStatus::Completed {
            item.estimate = Some(finished.counters);
        }
        item.status = finished.status;
        item.result = Some(finished.counters);
        item.errors = finished.errors;
        item.message = finished.message;
    }
}

fn worker_loop(shared: &Shared, mode: Mode, threads: usize) {
    loop {
        let next = shared.lock().start_next();
        let Some(job) = next else {
            break;
        };
        shared.emit_snapshot();

        let finished = run_job(shared, &job, mode, threads);
        shared.lock().finish(job.id, mode, finished);
        shared.emit_snapshot();
    }

    {
        let mut state = shared.lock();
        state.running = false;
        state.current = None;
    }
    shared.emit_snapshot();
}

fn run_job(shared: &Shared, job: &Job, mode: Mode, threads: usize) -> Finished {
    // Checked again right before the run: the folder may have changed since it was queued.
    if let Err(e) = shared.rules.check_deletable(&job.path) {
        return Finished {
            status: ItemStatus::Failed,
            counters: job.progress.snapshot(),
            errors: Vec::new(),
            message: Some(e.to_string()),
        };
    }

    let options = DeleteOptions {
        mode,
        keep_root: job.keep_root,
        threads,
    };

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let result = thread::scope(|scope| {
        scope.spawn(move || report_progress(shared, job, stop_rx));

        // A panic inside the engine must end as a failed job, not as a crashed app.
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            deleter::run(&job.path, &options, &job.progress)
        }));

        // Dropping the sender closes the channel, which stops the reporter.
        drop(stop_tx);
        result
    });

    let counters = job.progress.snapshot();
    let errors = job.progress.take_errors();
    let (status, message) = match result {
        Ok(Ok(outcome)) => (ItemStatus::from(outcome), None),
        Ok(Err(e)) => (ItemStatus::Failed, Some(e.to_string())),
        Err(_) => (ItemStatus::Failed, Some("internal error while deleting".into())),
    };

    Finished {
        status,
        counters,
        errors,
        message,
    }
}

/// Sends progress every `PROGRESS_INTERVAL` until the channel is closed.
fn report_progress(shared: &Shared, job: &Job, stop: mpsc::Receiver<()>) {
    while let Err(RecvTimeoutError::Timeout) = stop.recv_timeout(PROGRESS_INTERVAL) {
        shared.events.job_progress(job.id, &job.progress.snapshot());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;
    use std::fs;
    use std::path::Path;
    use std::time::Instant;

    #[derive(Default)]
    struct Recorder {
        changes: Mutex<usize>,
    }

    impl QueueEvents for Recorder {
        fn queue_changed(&self, _snapshot: &QueueSnapshot) {
            *self.changes.lock().unwrap() += 1;
        }

        fn job_progress(&self, _item_id: u64, _progress: &ProgressSnapshot) {}
    }

    fn queue(rules: SafetyRules) -> (JobQueue, Arc<Recorder>) {
        let recorder = Arc::new(Recorder::default());
        (JobQueue::new(rules, recorder.clone()), recorder)
    }

    fn no_rules() -> SafetyRules {
        SafetyRules::new(Vec::new(), Vec::new())
    }

    fn folder_with_files(parent: &Path, name: &str, files: usize) -> String {
        let dir = parent.join(name);
        fs::create_dir_all(dir.join("nested")).unwrap();
        for i in 0..files {
            fs::write(dir.join("nested").join(format!("{i}.txt")), b"12345").unwrap();
        }
        dir.to_string_lossy().into_owned()
    }

    fn wait_until_idle(queue: &JobQueue) -> QueueSnapshot {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let snapshot = queue.snapshot();
            if !snapshot.running {
                return snapshot;
            }
            assert!(Instant::now() < deadline, "queue did not finish in time");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn add_rejects_unsafe_missing_and_duplicate_paths() {
        let tmp = TestDir::new("queue-add");
        let folder = folder_with_files(&tmp.0, "cache", 1);
        let (queue, _) = queue(SafetyRules::new(Vec::new(), vec![tmp.0.clone()]));

        let outcome = queue.add(&[
            tmp.0.to_string_lossy().into_owned(),
            folder.clone(),
            folder.to_uppercase(),
            tmp.0.join("missing").to_string_lossy().into_owned(),
        ]);

        assert_eq!(outcome.added.len(), 1);
        assert_eq!(outcome.rejected.len(), 3);
        assert_eq!(queue.snapshot().items.len(), 1);
    }

    #[test]
    fn scan_then_delete_processes_every_item() {
        let tmp = TestDir::new("queue-run");
        let first = folder_with_files(&tmp.0, "first", 20);
        let second = folder_with_files(&tmp.0, "second", 30);
        let (queue, recorder) = queue(no_rules());
        queue.add(&[first.clone(), second.clone()]);

        queue.start(Mode::Scan, Some(4)).unwrap();
        let scanned = wait_until_idle(&queue);
        assert!(scanned.items.iter().all(|item| item.status == ItemStatus::Completed));
        assert_eq!(scanned.items[0].estimate.unwrap().files, 20);
        assert_eq!(scanned.items[1].estimate.unwrap().files, 30);
        assert!(Path::new(&first).exists());

        queue.start(Mode::Delete, Some(4)).unwrap();
        let deleted = wait_until_idle(&queue);
        assert!(deleted.items.iter().all(|item| item.status == ItemStatus::Completed));
        assert!(!Path::new(&first).exists());
        assert!(!Path::new(&second).exists());

        assert_eq!(queue.start(Mode::Delete, None), Err(QueueError::NothingToRun));
        assert!(*recorder.changes.lock().unwrap() > 0);
    }

    #[test]
    fn keep_root_option_is_applied() {
        let tmp = TestDir::new("queue-keep-root");
        let folder = folder_with_files(&tmp.0, "cache", 5);
        let (queue, _) = queue(no_rules());
        let id = queue.add(std::slice::from_ref(&folder)).added[0];

        queue.set_keep_root(id, true).unwrap();
        queue.start(Mode::Delete, Some(2)).unwrap();
        wait_until_idle(&queue);

        assert!(Path::new(&folder).exists());
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 0);
    }

    #[test]
    fn finished_items_can_be_removed_and_cleared() {
        let tmp = TestDir::new("queue-remove");
        let first = folder_with_files(&tmp.0, "first", 1);
        let second = folder_with_files(&tmp.0, "second", 1);
        let (queue, _) = queue(no_rules());
        let ids = queue.add(&[first, second]).added;

        queue.remove(ids[0]).unwrap();
        assert_eq!(queue.remove(ids[0]), Err(QueueError::NotFound(ids[0])));

        queue.start(Mode::Scan, Some(2)).unwrap();
        wait_until_idle(&queue);
        queue.clear_finished();
        assert!(queue.snapshot().items.is_empty());
    }
}
