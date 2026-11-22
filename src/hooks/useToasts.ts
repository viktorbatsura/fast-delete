import { useCallback, useEffect, useRef, useState } from "react";

export type ToastTone = "info" | "warning" | "error";

export interface Toast {
  id: number;
  title: string;
  detail?: string;
  tone: ToastTone;
}

const LIFETIME_MS = 5000;
const MAX_VISIBLE = 4;

export function useToasts() {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextId = useRef(1);
  const timers = useRef(new Map<number, number>());

  const dismiss = useCallback((id: number) => {
    window.clearTimeout(timers.current.get(id));
    timers.current.delete(id);
    setToasts((current) => current.filter((t) => t.id !== id));
  }, []);

  const push = useCallback(
    (title: string, tone: ToastTone = "info", detail?: string) => {
      const id = nextId.current++;
      setToasts((current) => [...current, { id, title, detail, tone }].slice(-MAX_VISIBLE));
      timers.current.set(id, window.setTimeout(() => dismiss(id), LIFETIME_MS));
    },
    [dismiss],
  );

  useEffect(() => {
    const pending = timers.current;
    return () => pending.forEach((timer) => window.clearTimeout(timer));
  }, []);

  return { toasts, push, dismiss };
}
