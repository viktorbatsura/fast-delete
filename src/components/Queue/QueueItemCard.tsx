import type { DeleteError } from "../../api/types";
import type { ItemView } from "../../queue/queueModel";
import { CloseIcon } from "../common/Icons";
import { Switch } from "../common/Switch";
import { formatInt, logDev } from "../../utils/format";
import styles from "./Queue.module.css";

const VISIBLE_ERRORS = 100;

interface QueueItemCardProps {
  view: ItemView;
  errors: DeleteError[];
  errorsOpen: boolean;
  onToggleErrors: () => void;
  onRemove: () => void;
  onKeepRootChange: (keepRoot: boolean) => void;
}

export function QueueItemCard({ view, errors, errorsOpen, onToggleErrors, onRemove, onKeepRootChange }: QueueItemCardProps) {
  const copyErrors = () => {
    const text = errors.map((e) => `${e.path}\t${e.message}`).join("\n");
    navigator.clipboard.writeText(text).catch(logDev);
  };

  return (
    <article className={`${styles.card} rise`}>
      <div className={styles.cardTop}>
        <span className={styles.cardName} title={view.name}>
          {view.name}
        </span>
        <span className={styles.status} data-tone={view.tone}>
          {view.statusLabel}
        </span>
      </div>
      <div className={styles.cardPath} title={view.path}>
        {view.path}
      </div>
      <div className={`${styles.cardSize} tabular`}>
        <span className={styles.sizeValue}>{view.size}</span>
        <span className={styles.sizeDetail}>{view.sizeDetail}</span>
      </div>

      {view.progress !== undefined && (
        <div className={styles.barTrack} data-indeterminate={view.progress === null}>
          <div
            className={styles.barFill}
            data-tone={view.tone}
            style={{ width: view.progress === null ? undefined : `${(view.progress * 100).toFixed(1)}%` }}
          />
        </div>
      )}
      {view.progressText && <div className={`${styles.cardNote} tabular`}>{view.progressText}</div>}
      {view.message && <div className={styles.cardError}>{view.message}</div>}

      {errors.length > 0 && (
        <>
          <button className={`${styles.linkButton} press`} data-tone="warn" onClick={onToggleErrors} aria-expanded={errorsOpen}>
            {errorsOpen ? "Hide details" : `Show ${formatInt(errors.length)} files that could not be processed`}
          </button>
          {errorsOpen && (
            <div className={`${styles.errorList} rise`}>
              {errors.slice(0, VISIBLE_ERRORS).map((error) => (
                <div key={error.path} className={styles.errorRow}>
                  <div className={styles.errorPath}>{error.path}</div>
                  <div className={styles.errorMessage}>{error.message}</div>
                </div>
              ))}
              {errors.length > VISIBLE_ERRORS && (
                <div className={styles.errorMessage}>and {formatInt(errors.length - VISIBLE_ERRORS)} more</div>
              )}
              <button className={`${styles.linkButton} press`} onClick={copyErrors}>
                Copy list
              </button>
            </div>
          )}
        </>
      )}

      {view.editable && (
        <div className={styles.cardActions}>
          <Switch checked={view.keepRoot} label="Keep empty folder" onChange={onKeepRootChange} />
          <button className={`${styles.iconButton} press`} aria-label={`Remove ${view.name} from queue`} onClick={onRemove}>
            <CloseIcon />
          </button>
        </div>
      )}
    </article>
  );
}
