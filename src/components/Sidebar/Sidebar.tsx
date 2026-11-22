import type { DriveInfo } from "../../api/types";
import { DriveIcon, DropIcon } from "../common/Icons";
import { formatBytes } from "../../utils/format";
import styles from "./Sidebar.module.css";

interface SidebarProps {
  drives: DriveInfo[];
  currentPath: string | null;
  onOpen: (path: string) => void;
}

function driveMeta(drive: DriveInfo): string {
  if (drive.totalBytes === null || drive.freeBytes === null) {
    return drive.kind === "network" ? "Network drive" : drive.kind === "optical" ? "Optical drive" : "Unavailable";
  }
  return `${formatBytes(drive.freeBytes)} free of ${formatBytes(drive.totalBytes)}`;
}

export function Sidebar({ drives, currentPath, onOpen }: SidebarProps) {
  const currentLetter = currentPath?.[0]?.toUpperCase();

  return (
    <nav className={styles.sidebar} aria-label="Drives">
      <div className={styles.heading}>Drives</div>
      {drives.map((drive) => {
        const selected = drive.letter === currentLetter;
        const used =
          drive.totalBytes && drive.freeBytes !== null ? 1 - drive.freeBytes / drive.totalBytes : null;
        return (
          <button
            key={drive.letter}
            className={`${styles.drive} press`}
            data-selected={selected}
            aria-current={selected ? "location" : undefined}
            onClick={() => onOpen(drive.path)}
          >
            <span className={styles.indicator} aria-hidden="true" />
            <DriveIcon className={styles.icon} />
            <span className={styles.body}>
              <span className={styles.title}>
                {drive.letter}:&nbsp; {drive.label ?? "Local Disk"}
              </span>
              {used !== null && (
                <span className={styles.track} aria-hidden="true">
                  <span className={styles.used} style={{ width: `${Math.round(used * 100)}%` }} />
                </span>
              )}
              <span className={styles.meta}>{driveMeta(drive)}</span>
            </span>
          </button>
        );
      })}
      <div className={styles.spacer} />
      <div className={styles.dropHint}>
        <DropIcon />
        <span>Drop folders from Explorer anywhere in this window to queue them.</span>
      </div>
    </nav>
  );
}
