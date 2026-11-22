import styles from "./Switch.module.css";

interface SwitchProps {
  checked: boolean;
  label: string;
  onChange: (checked: boolean) => void;
}

export function Switch({ checked, label, onChange }: SwitchProps) {
  return (
    <button role="switch" aria-checked={checked} className={`${styles.switch} press`} onClick={() => onChange(!checked)}>
      <span className={styles.track} data-on={checked}>
        <span className={styles.knob} />
      </span>
      {label}
    </button>
  );
}
