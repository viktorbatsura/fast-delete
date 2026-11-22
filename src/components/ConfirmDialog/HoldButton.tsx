import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import styles from "./ConfirmDialog.module.css";

interface HoldButtonProps {
  label: string;
  holdMs: number;
  describedBy?: string;
  onConfirm: () => void;
}

/**
 * Destructive confirmation: the action runs only after the button is held for `holdMs`.
 * Works with the mouse, touch and the keyboard (Space or Enter held down).
 */
export function HoldButton({ label, holdMs, describedBy, onConfirm }: HoldButtonProps) {
  const [holding, setHolding] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  const confirm = useRef(onConfirm);
  confirm.current = onConfirm;

  const start = () => {
    window.clearTimeout(timer.current);
    setHolding(true);
    timer.current = window.setTimeout(() => {
      setHolding(false);
      confirm.current();
    }, holdMs);
  };

  const cancel = () => {
    window.clearTimeout(timer.current);
    setHolding(false);
  };

  useEffect(() => () => window.clearTimeout(timer.current), []);

  const isHoldKey = (event: KeyboardEvent) => event.key === " " || event.key === "Enter";

  return (
    <button
      className={`${styles.hold} press`}
      data-holding={holding}
      style={{ ["--hold-duration" as string]: `${holdMs}ms` }}
      aria-describedby={describedBy}
      onPointerDown={(e) => {
        if (e.button === 0) start();
      }}
      onPointerUp={cancel}
      onPointerLeave={cancel}
      onPointerCancel={cancel}
      onKeyDown={(e) => {
        if (isHoldKey(e) && !e.repeat) {
          e.preventDefault();
          start();
        }
      }}
      onKeyUp={(e) => {
        if (isHoldKey(e)) cancel();
      }}
      onBlur={cancel}
    >
      <span>{label}</span>
      <span className={styles.holdFill} aria-hidden="true">
        {label}
      </span>
    </button>
  );
}
