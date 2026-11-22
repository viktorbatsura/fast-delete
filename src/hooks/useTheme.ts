import { useEffect } from "react";
import { applyTheme, onSystemThemeChange, type ThemeSetting } from "../theme/theme";

export function useTheme(setting: ThemeSetting): void {
  useEffect(() => {
    applyTheme(setting);
    if (setting !== "system") return;
    return onSystemThemeChange(() => applyTheme("system"));
  }, [setting]);
}
