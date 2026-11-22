// Pure view logic for the queue: turns backend state into what the panel shows.
import type { ItemStatus, QueueItem } from "../api/types";
import type { ProgressMap } from "../hooks/useQueue";
import { formatBytes, formatDuration, formatInt, formatRate } from "../utils/format";

export type Tone = "neutral" | "muted" | "accent" | "ok" | "warn" | "critical";

export interface ItemView {
  id: number;
  name: string;
  path: string;
  statusLabel: string;
  tone: Tone;
  size: string;
  sizeDetail: string;
  /** 0..1 when the total is known, `null` for an indeterminate bar, `undefined` for no bar. */
  progress?: number | null;
  progressText?: string;
  message?: string;
  editable: boolean;
  keepRoot: boolean;
}

export interface RunStats {
  mode: "delete" | "scan";
  files: number;
  bytes: number;
  elapsedMs: number;
  rate: number;
  etaMs: number | null;
  errors: number;
}

/** Mirrors `QueueItem::is_runnable` in Rust. */
export function isRunnable(item: QueueItem): boolean {
  switch (item.status) {
    case "idle":
    case "cancelled":
    case "failed":
    case "completedWithErrors":
      return true;
    case "completed":
      return item.mode === "scan";
    default:
      return false;
  }
}

export function isFinished(status: ItemStatus): boolean {
  return status === "completed" || status === "completedWithErrors" || status === "failed" || status === "cancelled";
}

function counters(item: QueueItem, progress: ProgressMap) {
  return item.status === "running" ? progress[item.id] : item.result ?? undefined;
}

export function itemView(item: QueueItem, progress: ProgressMap, busy: boolean): ItemView {
  const name = item.path.split(/[\\/]/).filter(Boolean).pop() ?? item.path;
  const live = counters(item, progress);
  const scanning = item.mode === "scan";
  const estimate = item.estimate;

  const view: ItemView = {
    id: item.id,
    name,
    path: item.path,
    statusLabel: "",
    tone: "neutral",
    size: estimate ? formatBytes(estimate.bytes) : "—",
    sizeDetail: estimate ? `${formatInt(estimate.files)} files` : "Scan to estimate",
    editable: !busy && isRunnable(item),
    keepRoot: item.keepRoot,
  };

  switch (item.status) {
    case "idle":
      view.statusLabel = estimate ? "Ready" : "Not scanned";
      break;
    case "queued":
      view.statusLabel = "Waiting";
      view.tone = "muted";
      break;
    case "running": {
      view.statusLabel = scanning ? "Scanning" : "Deleting";
      view.tone = "accent";
      const files = live?.files ?? 0;
      const total = !scanning && estimate ? estimate.files : 0;
      view.progress = total > 0 ? Math.min(1, files / total) : null;
      view.progressText = total > 0
        ? `${formatInt(files)} of ${formatInt(total)} files · ${formatBytes(live?.bytes ?? 0)}`
        : `${formatInt(files)} files · ${formatBytes(live?.bytes ?? 0)}`;
      break;
    }
    case "completed":
      view.statusLabel = scanning ? "Scanned" : "Deleted";
      view.tone = "ok";
      if (!scanning && live) {
        view.progress = 1;
        view.progressText = `${formatBytes(live.bytes)} freed · ${formatInt(live.files)} files`;
      }
      break;
    case "completedWithErrors":
      view.statusLabel = `${formatInt(live?.errors ?? item.errors.length)} not ${scanning ? "readable" : "deleted"}`;
      view.tone = "warn";
      if (live) {
        view.progress = 1;
        view.progressText = `${formatBytes(live.bytes)} ${scanning ? "found" : "freed"} · ${formatInt(live.files)} files`;
      }
      break;
    case "cancelled":
      view.statusLabel = "Stopped";
      view.tone = "muted";
      if (live) view.progressText = `Stopped after ${formatInt(live.files)} files · ${formatBytes(live.bytes)}`;
      break;
    case "failed":
      view.statusLabel = "Failed";
      view.tone = "critical";
      view.message = item.message ?? undefined;
      break;
  }

  return view;
}

/** Totals of the current or last run; `null` when there is nothing to summarize. */
export function runStats(items: QueueItem[], progress: ProgressMap, runIds: number[]): RunStats | null {
  const run = items.filter((item) => runIds.includes(item.id));
  if (run.length === 0) return null;

  let files = 0;
  let bytes = 0;
  let elapsedMs = 0;
  let errors = 0;
  let remaining = 0;
  let remainingKnown = true;

  for (const item of run) {
    const live = counters(item, progress);
    files += live?.files ?? 0;
    bytes += live?.bytes ?? 0;
    elapsedMs += live?.elapsedMs ?? 0;
    errors += live?.errors ?? 0;

    if (item.status === "queued" || item.status === "running") {
      if (item.estimate && item.mode === "delete") {
        remaining += Math.max(0, item.estimate.files - (live?.files ?? 0));
      } else {
        remainingKnown = false;
      }
    }
  }

  const rate = elapsedMs > 0 ? (files / elapsedMs) * 1000 : 0;
  const mode = run.find((item) => item.mode)?.mode ?? "delete";
  return {
    mode,
    files,
    bytes,
    elapsedMs,
    rate,
    etaMs: remainingKnown && rate > 0 ? (remaining / rate) * 1000 : null,
    errors,
  };
}

export function describeStats(stats: RunStats) {
  return {
    bytes: formatBytes(stats.bytes),
    files: formatInt(stats.files),
    rate: stats.rate > 0 ? formatRate(stats.rate) : "—",
    elapsed: formatDuration(stats.elapsedMs),
    eta: stats.etaMs === null ? "—" : formatDuration(stats.etaMs),
  };
}
