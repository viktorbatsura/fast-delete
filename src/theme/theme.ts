import { getCurrentWindow } from "@tauri-apps/api/window";
import { logDev } from "../utils/format";

export type ThemeSetting = "system" | "light" | "dark";

const darkQuery = () => window.matchMedia("(prefers-color-scheme: dark)");

export function resolveTheme(setting: ThemeSetting): "light" | "dark" {
  if (setting !== "system") return setting;
  return darkQuery().matches ? "dark" : "light";
}

/** Applies the theme to the page and to the native window (Mica follows the window theme). */
export function applyTheme(setting: ThemeSetting): void {
  document.documentElement.dataset.theme = resolveTheme(setting);
  getCurrentWindow()
    .setTheme(setting === "system" ? null : setting)
    .catch(logDev);
}

export function onSystemThemeChange(handler: () => void): () => void {
  const query = darkQuery();
  query.addEventListener("change", handler);
  return () => query.removeEventListener("change", handler);
}

interface UserAgentDataWithHints {
  getHighEntropyValues(hints: string[]): Promise<{ platform?: string; platformVersion?: string }>;
}

/**
 * Mica exists only on Windows 11. WebView2 reports Windows 11 as platformVersion 13 or higher.
 * Without Mica the page keeps a solid background, otherwise the transparent window would show the desktop.
 */
export async function detectMica(): Promise<void> {
  try {
    const uaData = (navigator as Navigator & { userAgentData?: UserAgentDataWithHints }).userAgentData;
    if (!uaData) return;
    const { platform, platformVersion } = await uaData.getHighEntropyValues(["platformVersion"]);
    const major = Number.parseInt(platformVersion?.split(".")[0] ?? "0", 10);
    if (platform === "Windows" && major >= 13) {
      document.documentElement.dataset.mica = "true";
    }
  } catch (error) {
    logDev(error);
  }
}
