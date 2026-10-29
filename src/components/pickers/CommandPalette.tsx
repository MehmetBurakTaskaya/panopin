import { useMemo, useState } from 'react';
import { t } from '../../i18n';
import { formatShortcut } from '../../shortcuts';
import { Dialog, fold, listKeys } from './Pickers';
import styles from './Pickers.module.css';

/** One action of the palette. `run` acts on what was selected when the palette opened. */
export interface PaletteCommand {
  id: string;
  label: string;
  /** In accelerator form ("Ctrl+Shift+T"), shown in the user's key names. */
  shortcut?: string;
  run: () => void;
}

/** The commands whose label holds every typed word, in their order. Pure, so it is tested. */
export function filterCommands(commands: PaletteCommand[], query: string): PaletteCommand[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return commands;
  return commands.filter((command) => {
    const label = fold(command.label);
    return words.every((word) => label.includes(word));
  });
}

/**
 * Ctrl+K: every action can be searched and run from the keyboard, each with its current shortcut
 * (D-063). ↑/↓ choose, Enter runs, Esc closes.
 */
export function CommandPalette({ commands, onClose }: { commands: PaletteCommand[]; onClose: () => void }) {
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const shown = useMemo(() => filterCommands(commands, query), [commands, query]);
  const run = (command: PaletteCommand | undefined) => {
    if (!command) return;
    onClose();
    command.run();
  };

  return (
    <Dialog title={t('palette.title')} onClose={onClose}>
      <div className={styles.palette}>
        <input
          className={styles.input}
          autoFocus
          value={query}
          placeholder={t('palette.search')}
          aria-label={t('palette.search')}
          aria-controls="palette-list"
          aria-activedescendant={shown[active] ? `palette-${shown[active].id}` : undefined}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
          onKeyDown={listKeys({
            count: shown.length,
            active,
            setActive,
            onEnter: () => run(shown[active]),
            onClose,
          })}
        />
        <ul id="palette-list" className={styles.list} role="listbox" aria-label={t('palette.title')}>
          {shown.map((command, i) => (
            <li
              key={command.id}
              id={`palette-${command.id}`}
              className={`${styles.option} ${styles.command}`}
              role="option"
              aria-selected={i === active}
              onMouseEnter={() => setActive(i)}
              onClick={() => run(command)}
            >
              <span>{command.label}</span>
              {command.shortcut && <kbd>{formatShortcut(command.shortcut)}</kbd>}
            </li>
          ))}
          {shown.length === 0 && <li className={`${styles.option} ${styles.create}`}>{t('palette.none')}</li>}
        </ul>
      </div>
    </Dialog>
  );
}
