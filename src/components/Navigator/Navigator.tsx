import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import type { DirListing } from "../../api/types";
import { ArrowUpIcon, CheckIcon, ChevronRightIcon, FolderIcon, LinkFolderIcon, PlusIcon } from "../common/Icons";
import { formatBytes, formatInt, splitPath } from "../../utils/format";
import { PathInput } from "./PathInput";
import styles from "./Navigator.module.css";

interface NavigatorProps {
  path: string | null;
  listing: DirListing | null;
  error: string | null;
  loading: boolean;
  isQueued: (path: string) => boolean;
  busy: boolean;
  onOpen: (path: string) => void;
  onUp: () => void;
  onAdd: (paths: string[]) => void;
}

export function Navigator({ path, listing, error, loading, isQueued, busy, onOpen, onUp, onAdd }: NavigatorProps) {
  const [selected, setSelected] = useState(0);
  const rowRefs = useRef<(HTMLDivElement | null)[]>([]);
  const listId = useId();
  const dirs = listing?.dirs ?? [];
  const crumbs = path ? splitPath(path) : [];
  const title = crumbs.length > 0 ? crumbs[crumbs.length - 1].label : "Fast Delete";

  // A new folder starts with the first row selected.
  useEffect(() => setSelected(0), [listing?.path]);

  useEffect(() => {
    rowRefs.current[selected]?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const onListKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const dir = dirs[selected];
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelected((i) => Math.min(dirs.length - 1, i + 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelected((i) => Math.max(0, i - 1));
    } else if (event.key === "Enter" && dir) {
      event.preventDefault();
      if (event.shiftKey) {
        if (!busy && !isQueued(dir.path)) onAdd([dir.path]);
      } else {
        onOpen(dir.path);
      }
    } else if (event.key === "Backspace") {
      event.preventDefault();
      onUp();
    }
  };

  return (
    <section className={styles.navigator} aria-label="Folders">
      <div className={styles.header}>
        <nav aria-label="Breadcrumb" className={styles.crumbs}>
          {crumbs.map((crumb, index) => (
            <span key={crumb.path} className={styles.crumb}>
              <button className={styles.crumbButton} onClick={() => onOpen(crumb.path)}>
                {crumb.label}
              </button>
              {index < crumbs.length - 1 && <ChevronRightIcon className={styles.crumbSeparator} />}
            </span>
          ))}
        </nav>
        <div className={styles.titleRow}>
          <button
            className={`${styles.iconButton} press`}
            aria-label="Up one level"
            onClick={onUp}
            disabled={crumbs.length <= 1}
          >
            <ArrowUpIcon />
          </button>
          <h1 className={styles.title}>{title}</h1>
        </div>
      </div>

      <PathInput currentPath={path} onNavigate={onOpen} onAdd={(p) => onAdd([p])} />

      <div className={styles.listHeader}>
        <span>Name</span>
        {listing && (
          <span className="tabular">
            {formatInt(listing.fileCount)} files · {formatBytes(listing.fileBytes)} at this level
          </span>
        )}
      </div>

      <div
        className={styles.list}
        role="listbox"
        aria-label="Subfolders"
        aria-describedby={`${listId}-hint`}
        aria-activedescendant={dirs[selected] ? `${listId}-${selected}` : undefined}
        aria-busy={loading}
        tabIndex={0}
        data-loading={loading}
        onKeyDown={onListKeyDown}
      >
        {error && (
          <div className={styles.state}>
            <div>Can't open this folder.</div>
            <div className={styles.stateDetail}>{error}</div>
            <button className={`${styles.secondaryButton} press`} onClick={onUp}>
              Go up
            </button>
          </div>
        )}
        {!error && listing && dirs.length === 0 && <div className={styles.state}>No subfolders here.</div>}

        {dirs.map((dir, index) => {
          const queued = isQueued(dir.path);
          return (
            <div
              key={dir.path}
              id={`${listId}-${index}`}
              ref={(el) => {
                rowRefs.current[index] = el;
              }}
              role="option"
              aria-selected={index === selected}
              className={styles.row}
              data-selected={index === selected}
              data-hidden={dir.isHidden}
              onClick={() => setSelected(index)}
              onDoubleClick={() => onOpen(dir.path)}
            >
              {dir.isLink ? <LinkFolderIcon /> : <FolderIcon />}
              <span className={styles.rowName}>{dir.name}</span>
              {dir.isLink && <span className={styles.rowDetail}>Link · removed as a link, never followed</span>}
              {!dir.isLink && dir.isHidden && <span className={styles.rowDetail}>Hidden</span>}
              <span className={styles.rowSpacer} />
              {queued ? (
                <span className={styles.inQueue}>
                  <CheckIcon />
                  In queue
                </span>
              ) : (
                <button
                  className={`${styles.addButton} press`}
                  tabIndex={-1}
                  disabled={busy}
                  onClick={(e) => {
                    e.stopPropagation();
                    onAdd([dir.path]);
                  }}
                >
                  <PlusIcon />
                  Add
                </button>
              )}
            </div>
          );
        })}

        {listing && listing.moreDirs > 0 && (
          <div className={styles.more}>
            and {formatInt(listing.moreDirs)} more folders — type a path to narrow down
          </div>
        )}
      </div>
      <div id={`${listId}-hint`} className={styles.keyHint}>
        Double-click or Enter opens · Shift+Enter adds to queue · Backspace goes up
      </div>
    </section>
  );
}
