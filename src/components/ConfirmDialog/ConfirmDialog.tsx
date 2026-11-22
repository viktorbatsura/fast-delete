import { useEffect, useId, useRef } from "react";
import type { QueueItem } from "../../api/types";
import { formatBytes } from "../../utils/format";
import { HoldButton } from "./HoldButton";
import styles from "./ConfirmDialog.module.css";

const HOLD_MS = 2000;

interface ConfirmDialogProps {
  items: QueueItem[];
  onConfirm: () => void;
  onCancel: () => void;
}

/** Second confirmation step: lists what will be deleted and requires holding the button. */
export function ConfirmDialog({ items, onConfirm, onCancel }: ConfirmDialogProps) {
  const titleId = useId();
  const hintId = useId();
  const cancelRef = useRef<HTMLButtonElement>(null);

  // Cancel gets focus first, so a stray Enter never starts a deletion.
  useEffect(() => {
    cancelRef.current?.focus();
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  const title = items.length > 1 ? `Delete ${items.length} folders permanently?` : "Delete this folder permanently?";
  const anyKeepRoot = items.some((item) => item.keepRoot);

  return (
    <div className={styles.scrim} onMouseDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div className={styles.dialog} role="alertdialog" aria-modal="true" aria-labelledby={titleId} aria-describedby={hintId}>
        <div className={styles.body}>
          <h2 id={titleId} className={styles.title}>
            {title}
          </h2>
          <p className={styles.text}>Files skip the Recycle Bin and cannot be restored.</p>
          <div className={styles.list}>
            {items.map((item) => (
              <div key={item.id} className={styles.listRow}>
                <span className={styles.listPath} title={item.path}>
                  {item.path}
                  {item.keepRoot && <span className={styles.keepTag}> · contents only</span>}
                </span>
                <span className={`${styles.listSize} tabular`}>
                  {item.estimate ? formatBytes(item.estimate.bytes) : "not scanned"}
                </span>
              </div>
            ))}
          </div>
          {anyKeepRoot && <p className={styles.note}>Folders marked “contents only” stay in place, empty.</p>}
        </div>
        <div className={styles.footer}>
          <div className={styles.buttons}>
            <button ref={cancelRef} className={`${styles.cancel} press`} onClick={onCancel}>
              Cancel
            </button>
            <HoldButton label="Hold to delete" holdMs={HOLD_MS} describedBy={hintId} onConfirm={onConfirm} />
          </div>
          <div id={hintId} className={styles.hint}>
            Press and hold for 2 seconds · release to cancel
          </div>
        </div>
      </div>
    </div>
  );
}
