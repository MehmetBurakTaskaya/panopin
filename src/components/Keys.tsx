import { formatShortcut } from '../shortcuts';

/** The ways to do one thing, a cap for each key. */
export function Keys({ keys, className }: { keys: readonly string[]; className?: string }) {
  return (
    <span className={className}>
      {keys.map((key) => (
        <kbd key={key}>{formatShortcut(key)}</kbd>
      ))}
    </span>
  );
}

/** A text with `{name}` places for keys ("{esc} to close"), each filled with its cap. */
export function KeyText({ text, keys }: { text: string; keys: Record<string, string> }) {
  return (
    <>
      {text.split(/(\{\w+\})/).map((part, i) => {
        const key = keys[part.slice(1, -1)];
        return part.startsWith('{') && key ? <kbd key={i}>{formatShortcut(key)}</kbd> : part;
      })}
    </>
  );
}
