//! Parallel folder deletion without a pre-scan.
//!
//! Every directory is read once, then its files are deleted and its subdirectories are processed
//! in parallel. A directory is removed as soon as it is empty. Links are removed, never followed.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::progress::Progress;

/// Deep trees recurse deeply; the default 2 MB stack of a worker thread is not always enough.
const WORKER_STACK_SIZE: usize = 8 * 1024 * 1024;

/// Antivirus or indexer handles can keep an emptied directory alive for a few milliseconds.
const RETRY_DELAYS_MS: [u64; 3] = [5, 20, 50];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Delete,
    /// Dry run: walks the tree and counts files and bytes without deleting anything.
    Scan,
}

#[derive(Debug, Clone)]
pub struct DeleteOptions {
    pub mode: Mode,
    /// Delete only the contents and keep the (empty) root folder.
    pub keep_root: bool,
    pub threads: usize,
}

impl DeleteOptions {
    /// One thread per logical CPU. Benchmarks on NVMe show a plateau at 8-16 threads
    /// and a slight slowdown beyond that (see docs/benchmarks).
    pub fn default_threads() -> usize {
        let cpus = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        cpus.clamp(4, 32)
    }
}

impl Default for DeleteOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Delete,
            keep_root: false,
            threads: Self::default_threads(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Completed,
    CompletedWithErrors,
    Cancelled,
}

#[derive(Debug)]
pub enum RunError {
    Root(String),
    ThreadPool(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root(msg) => write!(f, "cannot open folder: {msg}"),
            Self::ThreadPool(msg) => write!(f, "cannot start worker threads: {msg}"),
        }
    }
}

impl std::error::Error for RunError {}

/// Runs one job. Safety checks are the caller's responsibility (see `safety`).
pub fn run(root: &Path, options: &DeleteOptions, progress: &Progress) -> Result<Outcome, RunError> {
    let meta = fs::symlink_metadata(root).map_err(|e| RunError::Root(e.to_string()))?;

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(options.threads.max(1))
        .stack_size(WORKER_STACK_SIZE)
        .thread_name(|i| format!("fd-worker-{i}"))
        .build()
        .map_err(|e| RunError::ThreadPool(e.to_string()))?;

    let walker = Walker { options, progress };

    if meta.file_type().is_symlink() {
        // The root itself is a link: only the link goes away, its target is never touched.
        if !options.keep_root {
            let kind = if is_dir_link(&meta.file_type()) {
                EntryKind::DirLink
            } else {
                EntryKind::FileLink
            };
            walker.remove_entry(&Entry {
                path: root.to_path_buf(),
                size: 0,
                kind,
            });
        }
    } else if meta.is_dir() {
        pool.install(|| walker.process_dir(root, true));
    } else {
        return Err(RunError::Root("path is not a folder".into()));
    }

    let outcome = if progress.is_cancelled() {
        Outcome::Cancelled
    } else if progress.snapshot().errors > 0 {
        Outcome::CompletedWithErrors
    } else {
        Outcome::Completed
    };
    Ok(outcome)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    File,
    FileLink,
    DirLink,
}

#[derive(Debug)]
struct Entry {
    path: PathBuf,
    size: u64,
    kind: EntryKind,
}

/// Everything found in one directory. Directory links go to `files`: they are removed, not entered.
struct Listing {
    files: Vec<Entry>,
    subdirs: Vec<PathBuf>,
    /// False if some entries could not be read, so the directory cannot become empty.
    complete: bool,
}

struct Walker<'a> {
    options: &'a DeleteOptions,
    progress: &'a Progress,
}

impl Walker<'_> {
    /// Returns true when the directory was fully processed (deleted, or fully counted in scan mode).
    fn process_dir(&self, dir: &Path, is_root: bool) -> bool {
        if self.progress.is_cancelled() {
            return false;
        }

        let Some(listing) = self.read_listing(dir) else {
            return false;
        };

        let (files_ok, subdirs_ok) = rayon::join(
            || all_ok(listing.files.par_iter().map(|entry| self.remove_entry(entry))),
            || all_ok(listing.subdirs.par_iter().map(|sub| self.process_dir(sub, false))),
        );
        let emptied = files_ok && subdirs_ok && listing.complete;

        if is_root && self.options.keep_root {
            return emptied;
        }
        if !emptied || self.progress.is_cancelled() {
            return false;
        }
        self.remove_dir(dir)
    }

    fn read_listing(&self, dir: &Path) -> Option<Listing> {
        let reader = match fs::read_dir(dir) {
            Ok(reader) => reader,
            Err(e) => {
                self.progress.record_error(dir, e);
                return None;
            }
        };

        let mut listing = Listing {
            files: Vec::new(),
            subdirs: Vec::new(),
            complete: true,
        };

        for item in reader {
            let entry = match item {
                Ok(entry) => entry,
                Err(e) => {
                    self.progress.record_error(dir, e);
                    listing.complete = false;
                    continue;
                }
            };

            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(e) => {
                    self.progress.record_error(&path, e);
                    listing.complete = false;
                    continue;
                }
            };

            // On Windows `is_dir()` is false for junctions and directory symlinks.
            if file_type.is_dir() {
                listing.subdirs.push(path);
            } else if file_type.is_symlink() {
                let kind = if is_dir_link(&file_type) {
                    EntryKind::DirLink
                } else {
                    EntryKind::FileLink
                };
                listing.files.push(Entry { path, size: 0, kind });
            } else {
                // On Windows the size comes with the directory listing, no extra system call.
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                listing.files.push(Entry {
                    path,
                    size,
                    kind: EntryKind::File,
                });
            }
        }

        // The directory handle is closed here, before the directory itself is removed.
        Some(listing)
    }

    fn remove_entry(&self, entry: &Entry) -> bool {
        if self.progress.is_cancelled() {
            return false;
        }
        if self.options.mode == Mode::Scan {
            self.progress.add_file(entry.size);
            return true;
        }

        let result = match entry.kind {
            EntryKind::File | EntryKind::FileLink => {
                remove_with_retry(&entry.path, |p| fs::remove_file(p))
            }
            EntryKind::DirLink => remove_with_retry(&entry.path, |p| fs::remove_dir(p)),
        };

        match result {
            Ok(()) => {
                self.progress.add_file(entry.size);
                true
            }
            Err(e) => {
                self.progress.record_error(&entry.path, e);
                false
            }
        }
    }

    fn remove_dir(&self, dir: &Path) -> bool {
        if self.options.mode == Mode::Scan {
            self.progress.add_dir();
            return true;
        }

        match remove_with_retry(dir, |p| fs::remove_dir(p)) {
            Ok(()) => {
                self.progress.add_dir();
                true
            }
            Err(e) => {
                self.progress.record_error(dir, e);
                false
            }
        }
    }
}

/// Like `all()`, but without short-circuiting: every item is processed even after a failure.
fn all_ok(results: impl ParallelIterator<Item = bool>) -> bool {
    results.filter(|ok| !*ok).count() == 0
}

fn remove_with_retry<F>(path: &Path, remove: F) -> io::Result<()>
where
    F: Fn(&Path) -> io::Result<()>,
{
    let mut result = remove(path);

    if matches!(&result, Err(e) if e.kind() == io::ErrorKind::PermissionDenied)
        && clear_readonly(path)
    {
        result = remove(path);
    }

    for delay in RETRY_DELAYS_MS {
        match &result {
            Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => {
                thread::sleep(Duration::from_millis(delay));
                result = remove(path);
            }
            _ => break,
        }
    }

    // Someone else deleted it in the meantime: the goal is reached anyway.
    match result {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Returns true if the read-only attribute was set and has been cleared.
fn clear_readonly(path: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    let mut permissions = meta.permissions();
    if !permissions.readonly() {
        return false;
    }
    // On Windows this only clears FILE_ATTRIBUTE_READONLY.
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions).is_ok()
}

#[cfg(windows)]
fn is_dir_link(file_type: &fs::FileType) -> bool {
    use std::os::windows::fs::FileTypeExt;
    file_type.is_symlink_dir()
}

#[cfg(not(windows))]
fn is_dir_link(_file_type: &fs::FileType) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Temporary folder that removes itself when dropped, even if the test fails.
    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("fd-deleter-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct Totals {
        files: u64,
        bytes: u64,
        dirs: u64,
    }

    /// Creates 3 files per folder and 2 subfolders per level. Counts exclude `dir` itself.
    fn build_tree(dir: &Path, depth: u32) -> Totals {
        let mut totals = Totals::default();
        for i in 0..3 {
            fs::write(dir.join(format!("file{i}.txt")), b"0123456789").unwrap();
            totals.files += 1;
            totals.bytes += 10;
        }
        if depth > 0 {
            for i in 0..2 {
                let sub = dir.join(format!("sub{i}"));
                fs::create_dir(&sub).unwrap();
                let inner = build_tree(&sub, depth - 1);
                totals.files += inner.files;
                totals.bytes += inner.bytes;
                totals.dirs += inner.dirs + 1;
            }
        }
        totals
    }

    fn options(mode: Mode, keep_root: bool) -> DeleteOptions {
        DeleteOptions {
            mode,
            keep_root,
            threads: 4,
        }
    }

    #[test]
    fn scan_counts_everything_and_deletes_nothing() {
        let tmp = TestDir::new("scan");
        let root = tmp.0.join("target");
        fs::create_dir(&root).unwrap();
        let expected = build_tree(&root, 3);

        let progress = Progress::new();
        let outcome = run(&root, &options(Mode::Scan, false), &progress).unwrap();
        let snap = progress.snapshot();

        assert_eq!(outcome, Outcome::Completed);
        assert_eq!(snap.files, expected.files);
        assert_eq!(snap.bytes, expected.bytes);
        assert_eq!(snap.dirs, expected.dirs + 1);
        assert!(root.join("sub0").join("file0.txt").exists());
    }

    #[test]
    fn delete_removes_the_whole_tree() {
        let tmp = TestDir::new("delete");
        let root = tmp.0.join("target");
        fs::create_dir(&root).unwrap();
        let expected = build_tree(&root, 3);

        let progress = Progress::new();
        let outcome = run(&root, &options(Mode::Delete, false), &progress).unwrap();
        let snap = progress.snapshot();

        assert_eq!(outcome, Outcome::Completed);
        assert_eq!(snap.files, expected.files);
        assert_eq!(snap.dirs, expected.dirs + 1);
        assert!(!root.exists());
    }

    #[test]
    fn keep_root_leaves_an_empty_folder() {
        let tmp = TestDir::new("keep-root");
        let root = tmp.0.join("target");
        fs::create_dir(&root).unwrap();
        let expected = build_tree(&root, 2);

        let progress = Progress::new();
        run(&root, &options(Mode::Delete, true), &progress).unwrap();

        assert!(root.exists());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        assert_eq!(progress.snapshot().dirs, expected.dirs);
    }

    #[test]
    fn read_only_files_are_deleted() {
        let tmp = TestDir::new("read-only");
        let root = tmp.0.join("target");
        fs::create_dir(&root).unwrap();
        let file = root.join("locked.txt");
        fs::write(&file, b"x").unwrap();
        let mut permissions = fs::metadata(&file).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&file, permissions).unwrap();

        let progress = Progress::new();
        let outcome = run(&root, &options(Mode::Delete, false), &progress).unwrap();

        assert_eq!(outcome, Outcome::Completed);
        assert!(!root.exists());
    }

    #[test]
    fn cancelled_job_stops_and_keeps_files() {
        let tmp = TestDir::new("cancel");
        let root = tmp.0.join("target");
        fs::create_dir(&root).unwrap();
        build_tree(&root, 2);

        let progress = Progress::new();
        progress.cancel();
        let outcome = run(&root, &options(Mode::Delete, false), &progress).unwrap();

        assert_eq!(outcome, Outcome::Cancelled);
        assert!(root.join("file0.txt").exists());
    }

    #[test]
    fn missing_root_is_an_error() {
        let tmp = TestDir::new("missing");
        let progress = Progress::new();
        let result = run(&tmp.0.join("nope"), &options(Mode::Delete, false), &progress);
        assert!(matches!(result, Err(RunError::Root(_))));
    }

    #[cfg(windows)]
    #[test]
    fn junctions_are_removed_but_never_followed() {
        let tmp = TestDir::new("junction");
        let outside = tmp.0.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("precious.txt"), b"keep me").unwrap();

        let root = tmp.0.join("target");
        fs::create_dir(&root).unwrap();
        let link = root.join("link");

        // Junctions need no admin rights, unlike symbolic links.
        let created = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        assert!(created, "failed to create a test junction");

        let progress = Progress::new();
        let outcome = run(&root, &options(Mode::Delete, false), &progress).unwrap();

        assert_eq!(outcome, Outcome::Completed);
        assert!(!root.exists());
        assert!(outside.join("precious.txt").exists());
    }
}
