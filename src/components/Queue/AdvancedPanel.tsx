import type { Settings } from "../../hooks/useSettings";
import type { ThemeSetting } from "../../theme/theme";
import styles from "./Queue.module.css";

const THREAD_OPTIONS = [1, 2, 4, 8, 16, 32, 64];
const THEME_OPTIONS: { value: ThemeSetting; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

interface AdvancedPanelProps {
  settings: Settings;
  defaultThreads: number;
  onChange: (patch: Partial<Settings>) => void;
}

export function AdvancedPanel({ settings, defaultThreads, onChange }: AdvancedPanelProps) {
  return (
    <div className={`${styles.advanced} rise`}>
      <div className={styles.advancedRow}>
        <span>Worker threads</span>
        <span className={`${styles.muted} tabular`}>
          {settings.threads === null ? `Auto · ${defaultThreads}` : `${settings.threads} threads`}
        </span>
      </div>
      <div role="group" aria-label="Worker threads" className={styles.segmented} style={{ gridTemplateColumns: "repeat(8, minmax(0, 1fr))" }}>
        <button
          className={`${styles.segment} press`}
          aria-pressed={settings.threads === null}
          onClick={() => onChange({ threads: null })}
        >
          Auto
        </button>
        {THREAD_OPTIONS.map((threads) => (
          <button
            key={threads}
            className={`${styles.segment} press tabular`}
            aria-pressed={settings.threads === threads}
            onClick={() => onChange({ threads })}
          >
            {threads}
          </button>
        ))}
      </div>
      <div className={styles.muted}>On NVMe drives speed stops growing after 8–16 threads.</div>

      <div className={styles.advancedRow}>
        <span>Theme</span>
      </div>
      <div role="group" aria-label="Theme" className={styles.segmented} style={{ gridTemplateColumns: "repeat(3, minmax(0, 1fr))" }}>
        {THEME_OPTIONS.map((option) => (
          <button
            key={option.value}
            className={`${styles.segment} press`}
            aria-pressed={settings.theme === option.value}
            onClick={() => onChange({ theme: option.value })}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}
