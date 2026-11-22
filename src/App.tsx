import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { backend } from "./api/backend";
import { ConfirmDialog } from "./components/ConfirmDialog/ConfirmDialog";
import { DropOverlay, Toasts } from "./components/Feedback/Feedback";
import { Navigator } from "./components/Navigator/Navigator";
import { QueuePanel } from "./components/Queue/QueuePanel";
import { Sidebar } from "./components/Sidebar/Sidebar";
import { TitleBar } from "./components/TitleBar/TitleBar";
import { useDragDrop } from "./hooks/useDragDrop";
import { useNavigator } from "./hooks/useNavigator";
import { useQueue, type ActionError } from "./hooks/useQueue";
import { useSettings } from "./hooks/useSettings";
import { useTheme } from "./hooks/useTheme";
import { useToasts } from "./hooks/useToasts";
import { isRunnable } from "./queue/queueModel";
import { logDev, pathName } from "./utils/format";
import styles from "./App.module.css";

const FALLBACK_THREADS = 16;

export default function App() {
  const { settings, update } = useSettings();
  useTheme(settings.theme);

  const nav = useNavigator();
  const queue = useQueue();
  const toasts = useToasts();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [defaultThreads, setDefaultThreads] = useState(FALLBACK_THREADS);
  const { push } = toasts;

  useEffect(() => {
    backend
      .systemInfo()
      .then((info) => setDefaultThreads(info.defaultThreads))
      .catch(logDev);
  }, []);

  const queuedPaths = useMemo(
    () => new Set(queue.snapshot.items.map((item) => item.path.toLowerCase())),
    [queue.snapshot.items],
  );
  const isQueued = useCallback((path: string) => queuedPaths.has(path.toLowerCase()), [queuedPaths]);

  const report = useCallback(
    (error: ActionError) => {
      if (error) push("Action failed", "error", error);
    },
    [push],
  );

  const { add } = queue;
  const addPaths = useCallback(
    async (paths: string[]) => {
      const result = await add(paths);
      if (result === null) return;
      if (typeof result === "string") {
        push("Could not add to the queue", "error", result);
        return;
      }
      for (const rejection of result.rejected) {
        push(pathName(rejection.path), "warning", rejection.reason);
      }
    },
    [add, push],
  );

  const dragging = useDragDrop(addPaths);

  // After a run the free space changed and deleted folders are gone: refresh what is on screen.
  const { refreshDrives, open, path } = nav;
  const wasRunning = useRef(false);
  useEffect(() => {
    const running = queue.snapshot.running;
    if (wasRunning.current && !running) {
      refreshDrives();
      if (path) open(path);
    }
    wasRunning.current = running;
  }, [queue.snapshot.running, refreshDrives, open, path]);

  const runnable = queue.snapshot.items.filter(isRunnable);

  const startDelete = () => {
    setConfirmOpen(false);
    queue.start("delete", settings.threads).then(report);
  };

  return (
    <div className={styles.app}>
      <TitleBar />
      <div className={styles.body}>
        <Sidebar drives={nav.drives} currentPath={nav.path} onOpen={nav.open} />
        <main className={styles.main}>
          <Navigator
            path={nav.path}
            listing={nav.listing}
            error={nav.error}
            loading={nav.loading}
            isQueued={isQueued}
            busy={queue.snapshot.running}
            onOpen={nav.open}
            onUp={nav.up}
            onAdd={addPaths}
          />
          <QueuePanel
            snapshot={queue.snapshot}
            progress={queue.progress}
            runIds={queue.runIds}
            settings={settings}
            defaultThreads={defaultThreads}
            onSettingsChange={update}
            onRemove={(id) => queue.remove(id).then(report)}
            onKeepRootChange={(id, keep) => queue.setKeepRoot(id, keep).then(report)}
            onClearFinished={() => queue.clearFinished().then(report)}
            onScan={() => queue.start("scan", settings.threads).then(report)}
            onDelete={() => setConfirmOpen(true)}
            onStop={() => queue.cancel().then(report)}
          />
        </main>
      </div>

      {confirmOpen && runnable.length > 0 && (
        <ConfirmDialog items={runnable} onConfirm={startDelete} onCancel={() => setConfirmOpen(false)} />
      )}
      <DropOverlay visible={dragging} />
      <Toasts toasts={toasts.toasts} onDismiss={toasts.dismiss} />
    </div>
  );
}
