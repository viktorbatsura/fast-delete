# Fast Delete

Fast, parallel folder deletion for Windows — built with Rust and Tauri.

Deleting a huge folder (package caches, `node_modules`, build outputs) in Windows Explorer is slow:
it first scans the whole tree to estimate the size, moves everything to the Recycle Bin and works
almost sequentially. Fast Delete skips all of that and deletes files in parallel, which is much faster
on SSD/NVMe drives.

> **Status:** work in progress.

## Planned features

- Parallel deletion without a pre-scan, with live progress (files/s, bytes freed)
- Built-in folder navigator, path input and drag & drop from Explorer
- Batch queue for several folders
- Dry-run mode (count files and size without deleting)
- Two-step confirmation (dialog + hold-to-delete button)
- Error report for files that could not be deleted (locked, access denied)
- Protection against deleting system folders

## Build from source

Requirements: [Rust](https://rustup.rs) (MSVC toolchain), Node.js LTS, Visual Studio Build Tools with C++ workload.

```powershell
npm install
npm run tauri dev     # run in development mode
npm run tauri build   # build the installer
```

## License

[MIT](LICENSE)
