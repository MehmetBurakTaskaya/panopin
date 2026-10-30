import { useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react';
import { isRtl, t, type MessageKey } from '../../i18n';
import { formatShortcut } from '../../shortcuts';
import styles from './Shelf.module.css';

export interface MenuEntry {
  label: MessageKey;
  /** In accelerator form ("CommandOrControl+O"), shown in the user's key names. */
  shortcut: string;
  run: () => void;
}

export interface MenuPlacement {
  x: number;
  y: number;
}

const EDGE = 8;

/** The right-click menu: every shelf action for the clicked card or the selection. */
export function ContextMenu({ at, entries, onClose }: { at: MenuPlacement; entries: MenuEntry[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const items = useRef<(HTMLButtonElement | null)[]>([]);
  const [position, setPosition] = useState(at);
  const [active, setActive] = useState(0);

  // Keep the menu on screen and move focus into it.
  useLayoutEffect(() => {
    const menu = ref.current;
    if (!menu) return;
    const { width, height } = menu.getBoundingClientRect();
    // It opens towards the end of the line: right of the point, or left of it right to left.
    const x = isRtl() ? at.x - width : at.x;
    setPosition({
      x: Math.max(EDGE, Math.min(x, window.innerWidth - width - EDGE)),
      y: Math.max(EDGE, Math.min(at.y, window.innerHeight - height - EDGE)),
    });
    items.current[0]?.focus();
  }, [at]);

  const focusItem = (index: number) => {
    const next = (index + entries.length) % entries.length;
    setActive(next);
    items.current[next]?.focus();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    event.stopPropagation();
    const keys: Record<string, () => void> = {
      ArrowDown: () => focusItem(active + 1),
      ArrowUp: () => focusItem(active - 1),
      Home: () => focusItem(0),
      End: () => focusItem(entries.length - 1),
      Escape: onClose,
      Tab: onClose,
    };
    const handler = keys[event.key];
    if (handler) {
      event.preventDefault();
      handler();
    }
  };

  return (
    <>
      <div className={styles.menuCatcher} onPointerDown={onClose} onContextMenu={(event) => event.preventDefault()} />
      <div
        ref={ref}
        className={styles.menu}
        role="menu"
        style={{ left: position.x, top: position.y }}
        onKeyDown={onKeyDown}
        onContextMenu={(event) => event.preventDefault()}
      >
        {entries.map((entry, index) => (
          <button
            key={entry.label}
            ref={(el) => {
              items.current[index] = el;
            }}
            type="button"
            role="menuitem"
            tabIndex={index === active ? 0 : -1}
            className={styles.menuItem}
            onFocus={() => setActive(index)}
            onClick={() => {
              onClose();
              entry.run();
            }}
          >
            <span>{t(entry.label)}</span>
            <kbd>{entry.shortcut && formatShortcut(entry.shortcut)}</kbd>
          </button>
        ))}
      </div>
    </>
  );
}
