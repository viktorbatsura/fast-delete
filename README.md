# Fast Delete

[![CI](https://github.com/viktorbatsura/fast-delete/actions/workflows/ci.yml/badge.svg)](https://github.com/viktorbatsura/fast-delete/actions/workflows/ci.yml)

Fast, parallel folder deletion for Windows — built with Rust and Tauri.

Deleting a huge folder (package caches, `node_modules`, build outputs) in Windows Explorer is slow:
it first scans the whole tree to estimate the size, moves everything to the Recycle Bin and works
almost sequentially. Fast Delete skips all of that and deletes files in parallel.

## Performance

100,000 files in 3,000 folders on an NVMe SSD (Ryzen 7 7735HS, Windows 11, Defender on):

| Method                      | Time   | vs `rmdir` |
|-----------------------------|-------:|-----------:|
| `rmdir /s /q`               | 27.3 s |       1.0× |
| `std::fs::remove_dir_all`   | 20.1 s |       1.4× |
| **Fast Delete, 16 threads** | **5.0 s** |  **5.5×** |

Throughput grows up to 8–16 threads and then flattens out: past that point the disk and NTFS are the
bottleneck, not the CPU. By default Fast Delete uses one thread per logical CPU.

## Features

- Parallel deletion without a pre-scan, with live progress (freed space, files/s, time left)
- Built-in folder navigator with path autocomplete, plus drag & drop from Explorer
- Batch queue for several folders, optionally keeping the emptied root folder
- Scan mode: count files and size without deleting anything
- Two-step confirmation: a dialog and a hold-to-delete button
- Report of files that could not be deleted (locked by another process, access denied)
- Protection against deleting drive roots, system folders and the user profile
- Junctions and symbolic links are removed as links and never followed
- Light and dark theme with Windows 11 Mica

## Download

Get the latest installer from [Releases](https://github.com/viktorbatsura/fast-delete/releases).
It installs for the current user and needs no administrator rights.

The installer is not code-signed yet, so Windows SmartScreen may show a warning:
choose **More info → Run anyway**.

## Build from source

Requirements: [Rust](https://rustup.rs) (MSVC toolchain), Node.js LTS, Visual Studio Build Tools with the C++ workload.

```powershell
npm install
npm run tauri dev     # run in development mode
npm run tauri build   # build the installer
```

Run the tests with `cargo test --manifest-path src-tauri\Cargo.toml --lib`.

## License

[MIT](LICENSE)
