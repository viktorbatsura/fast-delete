import { useState } from "react";
import type { QueueSnapshot } from "../../api/types";
import type { ProgressMap } from "../../hooks/useQueue";
import type { Settings } from "../../hooks/useSettings";
import { describeStats, isFinished, isRunnable, itemView, runStats } from "../../queue/queueModel";
import { formatBytes, formatInt } from "../../utils/format";
import { ChevronRightIcon, SuccessIcon, WarningIcon } from "../common/Icons";
import { AdvancedPanel } from "./AdvancedPanel";
import { QueueItemCard } from "./QueueItemCard";
import styles from "./Queue.module.css";

interface QueuePanelProps {
  snapshot: QueueSnapshot;
  progress: ProgressMap;
  runIds: number[];
  settings: Settings;
  defaultThreads: number;
  onSettingsChange: (patch: Partial<Settings>) => void;
  onRemove: (id: number) => void;
  onKeepRootChange: (id: number, keepRoot: boolean) => void;
  onClearFinished: () => void;
  onScan: () => void;
  onDelete: () => void;
  onStop: () => void;
}

export function QueuePanel(props: QueuePanelProps) {
  const { snapshot, progress, runIds, settings, defaultThreads } = props;
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [errorsOpenId, setErrorsOpenId] = useState<number | null>(null);

  const running = snapshot.running;
  const runnable = snapshot.items.filter(isRunnable);
  const knownBytes = runnable.reduce((sum, item) => sum + (item.estimate?.bytes ?? 0), 0);
  const unscanned = runnable.filter((item) => !item.estimate).length;
  const stats = runStats(snapshot.items, progress, runIds);
  const statsText = stats ? describeStats(stats) : null;
  const canClear = !running && snapshot.items.some((item) => isFinished(item.status));
  const isScanRun = stats?.mode === "scan";

  const summary =
    runnable.length === 0
      ? "Nothing to delete yet"
      : `${runnable.length} ${runnable.length === 1 ? "folder" : "folders"} · ${formatBytes(knownBytes)}` +
        (unscanned > 0 ? ` + ${unscanned} not scanned` : "");

  return (
    <aside className={styles.panel} aria-label="Queue">
      <div className={styles.panelHeader}>
        <h2 className={styles.panelTitle}>Queue</h2>
        <span className={`${styles.count} tabular`}>{snapshot.items.length}</span>
        <span className={styles.flex} />
        {canClear && (
          <button className={`${styles.ghostButton} press`} onClick={props.onClearFinished}>
            Clear finished
          </button>
        )}
      </div>

      <div className={styles.items}>
        {snapshot.items.length === 0 && (
          <div className={styles.empty}>Queue is empty. Add folders from the list or drop them here.</div>
        )}
        {snapshot.items.map((item) => (
          <QueueItemCard
            key={item.id}
            view={itemView(item, progress, running)}
            errors={item.errors}
            errorsOpen={errorsOpenId === item.id}
            onToggleErrors={() => setErrorsOpenId((id) => (id === item.id ? null : item.id))}
            onRemove={() => props.onRemove(item.id)}
            onKeepRootChange={(keep) => props.onKeepRootChange(item.id, keep)}
          />
        ))}
      </div>

      <div className={styles.footer}>
        {running && statsText && (
          <>
            <div className={`${styles.stats} rise tabular`}>
              <div>
                <div className={styles.statLabel}>{isScanRun ? "Found" : "Freed"}</div>
                <div className={styles.statValue}>{statsText.bytes}</div>
              </div>
              <div>
                <div className={styles.statLabel}>Speed</div>
                <div className={styles.statValue}>{statsText.rate}</div>
              </div>
              <div>
                <div className={styles.statLabel}>Files</div>
                <div>{statsText.files}</div>
              </div>
              <div>
                <div className={styles.statLabel}>Time left</div>
                <div>{statsText.eta}</div>
              </div>
            </div>
            <button className={`${styles.secondaryButton} press`} onClick={props.onStop}>
              Stop
            </button>
          </>
        )}

        {!running && stats && statsText && runIds.length > 0 && (
          <div className={`${styles.banner} rise`} data-tone={stats.errors > 0 ? "warn" : "ok"}>
            {stats.errors > 0 ? <WarningIcon /> : <SuccessIcon />}
            <div>
              <div className={styles.bannerTitle}>
                {isScanRun ? `Found ${statsText.bytes}` : `${statsText.bytes} freed`} in {statsText.elapsed}
              </div>
              <div className={styles.bannerDetail}>
                {statsText.files} files
                {stats.errors > 0 && ` · ${formatInt(stats.errors)} could not be ${isScanRun ? "read" : "deleted"}`}
              </div>
            </div>
          </div>
        )}

        {!running && (
          <>
            <div className={styles.summary}>{summary}</div>
            <button
              className={`${styles.disclosure} press`}
              onClick={() => setAdvancedOpen((open) => !open)}
              aria-expanded={advancedOpen}
            >
              <ChevronRightIcon className={styles.chevron} data-open={advancedOpen} />
              Advanced
            </button>
            {advancedOpen && (
              <AdvancedPanel settings={settings} defaultThreads={defaultThreads} onChange={props.onSettingsChange} />
            )}
            <div className={styles.actions}>
              <button className={`${styles.secondaryButton} press`} onClick={props.onScan} disabled={runnable.length === 0}>
                Scan sizes
              </button>
              <button className={`${styles.dangerButton} press`} onClick={props.onDelete} disabled={runnable.length === 0}>
                {runnable.length > 1 ? `Delete ${runnable.length} folders` : "Delete folder"}
              </button>
            </div>
          </>
        )}
      </div>
    </aside>
  );
}
