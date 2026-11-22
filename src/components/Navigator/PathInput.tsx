import { useEffect, useId, useState, type KeyboardEvent } from "react";
import { backend } from "../../api/backend";
import { logDev } from "../../utils/format";
import styles from "./Navigator.module.css";

interface PathInputProps {
  currentPath: string | null;
  onNavigate: (path: string) => void;
  onAdd: (path: string) => void;
}

const SUGGEST_DELAY_MS = 120;

/** Path field with folder autocomplete: Tab completes, Enter opens, Escape closes the list. */
export function PathInput({ currentPath, onNavigate, onAdd }: PathInputProps) {
  const [value, setValue] = useState("");
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [active, setActive] = useState(-1);
  const [open, setOpen] = useState(false);
  const listId = useId();

  useEffect(() => {
    if (!/[\\/]/.test(value)) {
      setSuggestions([]);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      backend
        .suggestPaths(value)
        .then((list) => {
          if (cancelled) return;
          setSuggestions(list);
          setActive(-1);
        })
        .catch(logDev);
    }, SUGGEST_DELAY_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [value]);

  const showList = open && suggestions.length > 0;

  const complete = (path: string) => {
    setValue(`${path}\\`);
    setOpen(true);
  };

  const submit = () => {
    const target = active >= 0 ? suggestions[active] : value.trim();
    if (!target) return;
    onNavigate(target);
    setValue("");
    setOpen(false);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" && suggestions.length > 0) {
      event.preventDefault();
      setOpen(true);
      setActive((i) => (i + 1) % suggestions.length);
    } else if (event.key === "ArrowUp" && suggestions.length > 0) {
      event.preventDefault();
      setActive((i) => (i <= 0 ? suggestions.length - 1 : i - 1));
    } else if (event.key === "Tab" && showList) {
      event.preventDefault();
      complete(suggestions[Math.max(active, 0)]);
    } else if (event.key === "Enter") {
      event.preventDefault();
      submit();
    } else if (event.key === "Escape") {
      setOpen(false);
    }
  };

  const add = () => {
    const target = value.trim() || currentPath;
    if (target) onAdd(target);
    setValue("");
    setOpen(false);
  };

  return (
    <div className={styles.pathRow}>
      <div className={styles.pathField}>
        <label htmlFor={`${listId}-input`} className="visually-hidden">
          Folder path
        </label>
        <input
          id={`${listId}-input`}
          className={styles.pathInput}
          value={value}
          placeholder="Type or paste a folder path"
          spellCheck={false}
          autoComplete="off"
          role="combobox"
          aria-expanded={showList}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={active >= 0 ? `${listId}-${active}` : undefined}
          onChange={(e) => {
            setValue(e.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onBlur={() => setOpen(false)}
          onKeyDown={onKeyDown}
        />
        {showList && (
          <ul id={listId} role="listbox" className={styles.suggestions}>
            {suggestions.map((path, index) => (
              <li
                key={path}
                id={`${listId}-${index}`}
                role="option"
                aria-selected={index === active}
                className={styles.suggestion}
                data-active={index === active}
                // Keeps focus in the input so the list does not close before the click lands.
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => complete(path)}
              >
                {path}
              </li>
            ))}
          </ul>
        )}
      </div>
      <button className={`${styles.secondaryButton} press`} onClick={add} disabled={!value.trim() && !currentPath}>
        Add to queue
      </button>
    </div>
  );
}
