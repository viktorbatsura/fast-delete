import type { Toast } from "../../hooks/useToasts";
import { CloseIcon, DropIcon, ErrorIcon, WarningIcon } from "../common/Icons";
import styles from "./Feedback.module.css";

export function Toasts({ toasts, onDismiss }: { toasts: Toast[]; onDismiss: (id: number) => void }) {
  return (
    <div className={styles.toasts} role="status" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className={styles.toast}>
          {toast.tone === "error" ? <ErrorIcon /> : <WarningIcon />}
          <div className={styles.toastBody}>
            <div className={styles.toastTitle}>{toast.title}</div>
            {toast.detail && <div className={styles.toastDetail}>{toast.detail}</div>}
          </div>
          <button className={`${styles.toastClose} press`} aria-label="Dismiss" onClick={() => onDismiss(toast.id)}>
            <CloseIcon />
          </button>
        </div>
      ))}
    </div>
  );
}

export function DropOverlay({ visible }: { visible: boolean }) {
  if (!visible) return null;
  return (
    <div className={styles.dropOverlay} aria-hidden="true">
      <div className={styles.dropCard}>
        <DropIcon size={28} />
        Drop to add to the queue
      </div>
    </div>
  );
}
