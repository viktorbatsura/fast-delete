// Typed wrappers around Tauri commands and events. The UI talks to Rust only through this file.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AddOutcome,
  DirListing,
  DriveInfo,
  JobProgressPayload,
  Mode,
  QueueSnapshot,
  SystemInfo,
} from "./types";

export const backend = {
  systemInfo: () => invoke<SystemInfo>("system_info"),
  listDrives: () => invoke<DriveInfo[]>("list_drives"),
  listDir: (path: string) => invoke<DirListing>("list_dir", { path }),
  suggestPaths: (input: string) => invoke<string[]>("suggest_paths", { input }),

  queueSnapshot: () => invoke<QueueSnapshot>("queue_snapshot"),
  queueAdd: (paths: string[]) => invoke<AddOutcome>("queue_add", { paths }),
  queueRemove: (id: number) => invoke<void>("queue_remove", { id }),
  queueSetKeepRoot: (id: number, keepRoot: boolean) =>
    invoke<void>("queue_set_keep_root", { id, keepRoot }),
  queueClearFinished: () => invoke<void>("queue_clear_finished"),
  queueStart: (mode: Mode, threads?: number) =>
    invoke<void>("queue_start", { mode, threads: threads ?? null }),
  queueCancel: () => invoke<void>("queue_cancel"),
};

export const events = {
  onQueueChanged: (handler: (snapshot: QueueSnapshot) => void): Promise<UnlistenFn> =>
    listen<QueueSnapshot>("queue-changed", (event) => handler(event.payload)),
  onJobProgress: (handler: (payload: JobProgressPayload) => void): Promise<UnlistenFn> =>
    listen<JobProgressPayload>("job-progress", (event) => handler(event.payload)),
};
