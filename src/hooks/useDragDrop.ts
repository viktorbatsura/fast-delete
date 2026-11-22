import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { logDev } from "../utils/format";

/** Files and folders dragged from Explorer onto the window. Returns whether a drag is over the window. */
export function useDragDrop(onDrop: (paths: string[]) => void): boolean {
  const [dragging, setDragging] = useState(false);
  // Keeps the subscription stable while always calling the latest handler.
  const handler = useRef(onDrop);
  handler.current = onDrop;

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          setDragging(true);
        } else if (payload.type === "leave") {
          setDragging(false);
        } else if (payload.type === "drop") {
          setDragging(false);
          if (payload.paths.length > 0) handler.current(payload.paths);
        }
      })
      .then((fn) => (disposed ? fn() : (unlisten = fn)))
      .catch(logDev);

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return dragging;
}
