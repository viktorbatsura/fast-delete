const KB = 1024;
const MB = KB * 1024;
const GB = MB * 1024;

export function formatBytes(bytes: number): string {
  if (bytes >= GB) return `${(bytes / GB).toFixed(1)} GB`;
  if (bytes >= MB) return `${Math.round(bytes / MB)} MB`;
  if (bytes >= KB) return `${Math.round(bytes / KB)} KB`;
  return `${bytes} B`;
}

export function formatInt(value: number): string {
  return Math.round(value).toLocaleString("en-US");
}

export function formatRate(filesPerSecond: number): string {
  return `${(filesPerSecond / 1000).toFixed(1)}k files/s`;
}

export function formatDuration(ms: number): string {
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)} s`;
  return `${Math.floor(seconds / 60)} min ${Math.round(seconds % 60)} s`;
}

export function pathName(path: string): string {
  const parts = path.replace(/[\\/]+$/, "").split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/** Windows paths are case-insensitive. */
export function samePath(a: string, b: string): boolean {
  return a.replace(/[\\/]+$/, "").toLowerCase() === b.replace(/[\\/]+$/, "").toLowerCase();
}

export interface Crumb {
  label: string;
  path: string;
}

/** `C:\Users\me` → `C:` (C:\), `Users` (C:\Users), `me` (C:\Users\me). */
export function splitPath(path: string): Crumb[] {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  if (parts.length === 0) return [];

  const crumbs: Crumb[] = [{ label: parts[0], path: `${parts[0]}\\` }];
  let current = parts[0];
  for (const part of parts.slice(1)) {
    current = `${current}\\${part}`;
    crumbs.push({ label: part, path: current });
  }
  return crumbs;
}

/** Errors only reach the console during local development. */
export function logDev(error: unknown): void {
  if (import.meta.env.DEV) {
    console.error(error);
  }
}
