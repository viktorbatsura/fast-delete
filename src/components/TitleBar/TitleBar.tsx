import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { AppIcon } from "../common/Icons";
import { logDev } from "../../utils/format";
import styles from "./TitleBar.module.css";

/** Custom title bar: the window has no native decorations so Mica runs edge to edge. */
export function TitleBar() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    const sync = () => appWindow.isMaximized().then((value) => !disposed && setMaximized(value)).catch(logDev);

    sync();
    appWindow
      .onResized(sync)
      .then((fn) => (disposed ? fn() : (unlisten = fn)))
      .catch(logDev);

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const appWindow = getCurrentWindow();
  const call = (action: () => Promise<void>) => () => {
    action().catch(logDev);
  };

  return (
    <header className={styles.bar} data-tauri-drag-region>
      <div className={styles.brand} data-tauri-drag-region>
        <AppIcon />
        <span data-tauri-drag-region>Fast Delete</span>
      </div>
      <div className={styles.controls}>
        <button className={styles.caption} aria-label="Minimize" onClick={call(() => appWindow.minimize())}>
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
            <path d="M0 5h10" stroke="var(--caption-glyph)" strokeWidth="1" />
          </svg>
        </button>
        <button
          className={styles.caption}
          aria-label={maximized ? "Restore" : "Maximize"}
          onClick={call(() => appWindow.toggleMaximize())}
        >
          {maximized ? (
            <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
              <rect x="0.5" y="2.5" width="7" height="7" rx="1.2" fill="none" stroke="var(--caption-glyph)" />
              <path d="M2.5 2.5V1.8A1.3 1.3 0 013.8.5h4.4A1.3 1.3 0 019.5 1.8v4.4a1.3 1.3 0 01-1.3 1.3h-.7" fill="none" stroke="var(--caption-glyph)" />
            </svg>
          ) : (
            <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
              <rect x="0.5" y="0.5" width="9" height="9" rx="1.5" fill="none" stroke="var(--caption-glyph)" />
            </svg>
          )}
        </button>
        <button className={`${styles.caption} ${styles.close}`} aria-label="Close" onClick={call(() => appWindow.close())}>
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
            <path d="M0 0l10 10M10 0L0 10" stroke="var(--caption-glyph)" strokeWidth="1" />
          </svg>
        </button>
      </div>
    </header>
  );
}
