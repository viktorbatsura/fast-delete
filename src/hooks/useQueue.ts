import { useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { backend, events } from "../api/backend";
import type { AddOutcome, Mode, ProgressSnapshot, QueueSnapshot } from "../api/types";
import { logDev } from "../utils/format";

export type ProgressMap = Record<number, ProgressSnapshot>;

/** Result of an action: `null` on success, otherwise a message for the user. */
export type ActionError = string | null;

async function run(action: () => Promise<unknown>): Promise<ActionError> {
  try {
    await action();
    return null;
  } catch (error) {
    logDev(error);
    return String(error);
  }
}

export function useQueue() {
  const [snapshot, setSnapshot] = useState<QueueSnapshot>({ items: [], running: false });
  const [progress, setProgress] = useState<ProgressMap>({});
  /** Ids of the items that belong to the current (or last) run, for the run summary. */
  const [runIds, setRunIds] = useState<number[]>([]);
  const wasRunning = useRef(false);

  const applySnapshot = useCallback((next: QueueSnapshot) => {
    setSnapshot(next);

    // Live counters only make sense for the running item; older values would flash on restart.
    const runningIds = new Set(next.items.filter((i) => i.status === "running").map((i) => i.id));
    setProgress((current) => {
      const kept: ProgressMap = {};
      for (const id of runningIds) {
        if (current[id]) kept[id] = current[id];
      }
      return kept;
    });

    if (next.running && !wasRunning.current) {
      setRunIds(next.items.filter((i) => i.status === "queued" || i.status === "running").map((i) => i.id));
    }
    wasRunning.current = next.running;
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    const keep = (unlisten: UnlistenFn) => (disposed ? unlisten() : unlisteners.push(unlisten));

    backend.queueSnapshot().then((s) => !disposed && applySnapshot(s)).catch(logDev);
    events.onQueueChanged(applySnapshot).then(keep).catch(logDev);
    events
      .onJobProgress(({ itemId, progress: p }) => setProgress((current) => ({ ...current, [itemId]: p })))
      .then(keep)
      .catch(logDev);

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [applySnapshot]);

  const add = useCallback(async (paths: string[]): Promise<AddOutcome | ActionError> => {
    try {
      return await backend.queueAdd(paths);
    } catch (error) {
      logDev(error);
      return String(error);
    }
  }, []);

  const actions = {
    add,
    remove: (id: number) => run(() => backend.queueRemove(id)),
    setKeepRoot: (id: number, keepRoot: boolean) => run(() => backend.queueSetKeepRoot(id, keepRoot)),
    clearFinished: () => {
      setRunIds([]);
      return run(() => backend.queueClearFinished());
    },
    start: (mode: Mode, threads: number | null) => run(() => backend.queueStart(mode, threads ?? undefined)),
    cancel: () => run(() => backend.queueCancel()),
  };

  return { snapshot, progress, runIds, ...actions };
}
