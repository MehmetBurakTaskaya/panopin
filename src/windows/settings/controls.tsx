import { useId, type ReactNode } from 'react';
import styles from './Settings.module.css';

/** A titled group of rows. */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className={styles.section}>
      <h2>{title}</h2>
      <div className={styles.group}>{children}</div>
    </section>
  );
}

interface RowProps {
  label: string;
  hint?: ReactNode;
  children: ReactNode;
  /** The control the label names; a row of buttons has none. */
  htmlFor?: string;
}

export function Row({ label, hint, children, htmlFor }: RowProps) {
  return (
    <div className={styles.row}>
      <label htmlFor={htmlFor} className={styles.label}>
        <span>{label}</span>
        {hint && <small>{hint}</small>}
      </label>
      <div className={styles.control}>{children}</div>
    </div>
  );
}

/** An on/off switch; it applies at once. */
export function Toggle(props: { label: string; hint?: ReactNode; checked: boolean; disabled?: boolean; onChange: (on: boolean) => void }) {
  const id = useId();
  return (
    <Row label={props.label} hint={props.hint} htmlFor={id}>
      <input
        id={id}
        type="checkbox"
        role="switch"
        className={styles.switch}
        checked={props.checked}
        disabled={props.disabled}
        onChange={(e) => props.onChange(e.target.checked)}
      />
    </Row>
  );
}

/** A choice from a short list. Values are strings; callers map them back. */
export function Choice(props: {
  label: string;
  hint?: ReactNode;
  value: string;
  options: [string, string][];
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  const id = useId();
  return (
    <Row label={props.label} hint={props.hint} htmlFor={id}>
      <select id={id} value={props.value} disabled={props.disabled} onChange={(e) => props.onChange(e.target.value)}>
        {props.options.map(([value, label]) => (
          <option key={value} value={value}>
            {label}
          </option>
        ))}
      </select>
    </Row>
  );
}
