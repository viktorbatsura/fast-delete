// Mirrors the Rust types serialized by the backend (camelCase). Keep in sync with src-tauri/src.

export type Mode = "delete" | "scan";

export type DriveKind = "fixed" | "removable" | "network" | "optical" | "ramDisk" | "unknown";

export interface DriveInfo {
  path: string;
  letter: string;
  kind: DriveKind;
  label: string | null;
  fileSystem: string | null;
  totalBytes: number | null;
  freeBytes: number | null;
}

export interface DirEntryInfo {
  name: string;
  path: string;
  isLink: boolean;
  isHidden: boolean;
}

export interface DirListing {
  path: string;
  parent: string | null;
  dirs: DirEntryInfo[];
  moreDirs: number;
  fileCount: number;
  fileBytes: number;
}

export interface ProgressSnapshot {
  files: number;
  dirs: number;
  bytes: number;
  errors: number;
  elapsedMs: number;
}

export interface DeleteError {
  path: string;
  message: string;
}

export type ItemStatus =
  | "idle"
  | "queued"
  | "running"
  | "completed"
  | "completedWithErrors"
  | "cancelled"
  | "failed";

export interface QueueItem {
  id: number;
  path: string;
  keepRoot: boolean;
  status: ItemStatus;
  mode: Mode | null;
  estimate: ProgressSnapshot | null;
  result: ProgressSnapshot | null;
  errors: DeleteError[];
  message: string | null;
}

export interface QueueSnapshot {
  items: QueueItem[];
  running: boolean;
}

export interface Rejection {
  path: string;
  reason: string;
}

export interface AddOutcome {
  added: number[];
  rejected: Rejection[];
}

export interface JobProgressPayload {
  itemId: number;
  progress: ProgressSnapshot;
}

export interface SystemInfo {
  appVersion: string;
  logicalCpus: number;
  defaultThreads: number;
}
