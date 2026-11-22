//! Deletion benchmark: Fast Delete with different thread counts vs `rmdir /s /q`
//! and `std::fs::remove_dir_all`.
//!
//! A fresh tree is generated before every measured run, only the deletion itself is timed.
//!
//! ```text
//! cargo run --profile bench --example bench -- --files 100000 --threads 1,2,4,8,16,32,64 --rmdir --std --out docs/benchmarks/results.jsonl
//! cargo run --profile bench --example bench -- --threads none --rmdir
//! ```

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use fast_delete_lib::engine::deleter::{self, DeleteOptions, Mode};
use fast_delete_lib::engine::progress::Progress;
use rayon::prelude::*;
use serde::Serialize;

// Shape similar to a package cache: many small packages with a few folders of small files.
const SUBDIRS_PER_PACKAGE: usize = 2;
const FILES_PER_SUBDIR: usize = 50;
const FILE_SIZE: usize = 1024;

struct Args {
    dir: PathBuf,
    files: usize,
    threads: Vec<usize>,
    rmdir: bool,
    std_remove: bool,
    label: String,
    out: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
enum Method {
    FastDelete(usize),
    Rmdir,
    StdRemoveDirAll,
}

impl Method {
    fn name(self) -> &'static str {
        match self {
            Self::FastDelete(_) => "fast-delete",
            Self::Rmdir => "rmdir /s /q",
            Self::StdRemoveDirAll => "std::fs::remove_dir_all",
        }
    }

    fn threads(self) -> usize {
        match self {
            Self::FastDelete(threads) => threads,
            Self::Rmdir | Self::StdRemoveDirAll => 1,
        }
    }
}

struct Tree {
    files: u64,
    dirs: u64,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    label: String,
    method: &'static str,
    threads: usize,
    files: u64,
    dirs: u64,
    bytes: u64,
    seconds: f64,
    files_per_sec: f64,
    errors: u64,
    fully_deleted: bool,
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(2);
        }
    };

    let mut methods: Vec<Method> = args.threads.iter().map(|&t| Method::FastDelete(t)).collect();
    if args.rmdir {
        methods.push(Method::Rmdir);
    }
    if args.std_remove {
        methods.push(Method::StdRemoveDirAll);
    }

    let root = args.dir.join("tree");
    let _ = fs::remove_dir_all(&args.dir);

    println!("Benchmark folder: {}", root.display());
    println!("{:<26} {:>8} {:>10} {:>14}", "method", "threads", "seconds", "files/s");

    let mut records = Vec::new();
    for method in methods {
        let tree = build_tree(&root, args.files);
        let record = measure(method, &root, &tree, &args.label);
        println!(
            "{:<26} {:>8} {:>10.2} {:>14.0}{}",
            record.method,
            record.threads,
            record.seconds,
            record.files_per_sec,
            if record.fully_deleted { "" } else { "  (incomplete!)" }
        );
        records.push(record);
        let _ = fs::remove_dir_all(&root);
    }

    let _ = fs::remove_dir_all(&args.dir);

    if let Some(out) = &args.out {
        if let Err(e) = append_records(out, &records) {
            eprintln!("error: cannot write {}: {e}", out.display());
            std::process::exit(1);
        }
        println!("Results appended to {}", out.display());
    }
}

fn measure(method: Method, root: &Path, tree: &Tree, label: &str) -> Record {
    let start = Instant::now();
    let errors = match method {
        Method::FastDelete(threads) => {
            let progress = Progress::new();
            let options = DeleteOptions {
                mode: Mode::Delete,
                keep_root: false,
                threads,
            };
            deleter::run(root, &options, &progress).expect("deleter failed to start");
            progress.snapshot().errors
        }
        Method::Rmdir => {
            let success = Command::new("cmd")
                .args(["/C", "rmdir", "/S", "/Q"])
                .arg(root)
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            u64::from(!success)
        }
        Method::StdRemoveDirAll => u64::from(fs::remove_dir_all(root).is_err()),
    };
    let seconds = start.elapsed().as_secs_f64();

    Record {
        label: label.to_string(),
        method: method.name(),
        threads: method.threads(),
        files: tree.files,
        dirs: tree.dirs,
        bytes: tree.bytes,
        seconds,
        files_per_sec: tree.files as f64 / seconds,
        errors,
        fully_deleted: !root.exists(),
    }
}

fn build_tree(root: &Path, files: usize) -> Tree {
    let per_package = SUBDIRS_PER_PACKAGE * FILES_PER_SUBDIR;
    let packages = files.div_ceil(per_package).max(1);
    let content = vec![b'x'; FILE_SIZE];

    fs::create_dir_all(root).expect("cannot create benchmark folder");

    // Generation runs in parallel too, otherwise preparing each run would take longer than the run.
    (0..packages).into_par_iter().for_each(|package| {
        for part in 0..SUBDIRS_PER_PACKAGE {
            let dir = root
                .join(format!("package-{package:05}"))
                .join(format!("part-{part}"));
            fs::create_dir_all(&dir).expect("cannot create folder");
            for file in 0..FILES_PER_SUBDIR {
                fs::write(dir.join(format!("file-{file:03}.js")), &content)
                    .expect("cannot write file");
            }
        }
    });

    let files = (packages * per_package) as u64;
    Tree {
        files,
        dirs: (packages * (1 + SUBDIRS_PER_PACKAGE) + 1) as u64,
        bytes: files * FILE_SIZE as u64,
    }
}

fn append_records(out: &Path, records: &[Record]) -> std::io::Result<()> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(out)?;
    for record in records {
        let line = serde_json::to_string(record).map_err(std::io::Error::other)?;
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        dir: std::env::temp_dir().join("fd-bench"),
        files: 100_000,
        threads: vec![1, 2, 4, 8, 16, 32, 64],
        rmdir: false,
        std_remove: false,
        label: "default".to_string(),
        out: None,
    };

    let mut raw = std::env::args().skip(1);
    while let Some(flag) = raw.next() {
        match flag.as_str() {
            "--dir" => args.dir = PathBuf::from(value(&mut raw, "--dir")?),
            "--files" => {
                args.files = value(&mut raw, "--files")?
                    .parse()
                    .map_err(|e| format!("--files: {e}"))?;
            }
            "--threads" => args.threads = parse_list(&value(&mut raw, "--threads")?)?,
            "--rmdir" => args.rmdir = true,
            "--std" => args.std_remove = true,
            "--label" => args.label = value(&mut raw, "--label")?,
            "--out" => args.out = Some(PathBuf::from(value(&mut raw, "--out")?)),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(args)
}

fn value(raw: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    raw.next().ok_or_else(|| format!("{flag} needs a value"))
}

fn parse_list(list: &str) -> Result<Vec<usize>, String> {
    if list == "none" {
        return Ok(Vec::new());
    }
    list.split(',')
        .map(|part| {
            part.trim()
                .parse::<usize>()
                .map_err(|e| format!("--threads: {e}"))
        })
        .collect()
}
