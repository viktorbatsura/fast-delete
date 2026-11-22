import { useCallback, useState } from "react";
import type { ThemeSetting } from "../theme/theme";

export interface Settings {
  theme: ThemeSetting;
  /** `null` means automatic: one thread per logical CPU, decided by the backend. */
  threads: number | null;
}

const STORAGE_KEY = "fast-delete.settings";
const DEFAULTS: Settings = { theme: "system", threads: null };

// Storage can be unavailable or hold garbage from an older version: fall back to defaults.
export function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    const parsed = JSON.parse(raw) as Partial<Settings>;
    return {
      theme: parsed.theme === "light" || parsed.theme === "dark" ? parsed.theme : "system",
      threads: typeof parsed.threads === "number" && parsed.threads > 0 ? parsed.threads : null,
    };
  } catch {
    return DEFAULTS;
  }
}

function saveSettings(settings: Settings): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Not persisting a preference is harmless.
  }
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings>(loadSettings);

  const update = useCallback((patch: Partial<Settings>) => {
    setSettings((current) => {
      const next = { ...current, ...patch };
      saveSettings(next);
      return next;
    });
  }, []);

  return { settings, update };
}
