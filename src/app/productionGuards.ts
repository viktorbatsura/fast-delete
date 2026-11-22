/**
 * Makes the release build behave like a native app instead of a web page:
 * no browser context menu and no browser shortcuts (reload, print, find, save).
 * In development everything stays available for debugging.
 */
export function installProductionGuards(): void {
  if (import.meta.env.DEV) return;

  document.addEventListener("contextmenu", (event) => {
    const target = event.target as HTMLElement | null;
    // Text fields keep the native Cut / Copy / Paste menu.
    if (target?.closest("input, textarea, [contenteditable='true']")) return;
    event.preventDefault();
  });

  document.addEventListener("keydown", (event) => {
    const key = event.key.toLowerCase();
    const ctrl = event.ctrlKey || event.metaKey;
    const browserShortcut =
      key === "f5" ||
      key === "f7" ||
      (ctrl && ["r", "p", "f", "g", "s", "u", "j"].includes(key)) ||
      (ctrl && event.shiftKey && ["i", "c", "j"].includes(key));
    if (browserShortcut) event.preventDefault();
  });
}
