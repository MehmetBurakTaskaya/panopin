import { useEffect, useState } from 'react';
import { t, type MessageKey } from '../../i18n';
import { setShortcut, suspendShortcuts, unavailableShortcuts, type Settings, type ShortcutAction, type Shortcuts } from '../../ipc';
import { acceleratorFrom, clashesWithAltGr, formatShortcut } from '../../shortcuts';
import { useSettings } from '../../state/settings';
import { Section } from './controls';
import styles from './Settings.module.css';

const ACTIONS: [ShortcutAction, keyof Shortcuts, MessageKey][] = [
  ['toggleShelf', 'shelf', 'settings.shortcut.shelf'],
  ['openShelfSearch', 'shelfSearch', 'settings.shortcut.shelfSearch'],
  ['snip', 'snip', 'settings.shortcut.snip'],
  ['openLibrary', 'library', 'settings.shortcut.library'],
  ['copyLastImage', 'copyLastImage', 'settings.shortcut.copyLastImage'],
  ['pauseCapture', 'pauseCapture', 'settings.shortcut.pauseCapture'],
];

/** The approved defaults (D-020). */
const DEFAULTS: Shortcuts = {
  shelf: 'Super+Alt+V',
  shelfSearch: 'Super+Alt+C',
  snip: 'Super+Alt+S',
  library: null,
  copyLastImage: null,
  pauseCapture: null,
};

/**
 * Listens for one shortcut. Panopin's own shortcuts are released meanwhile and come back when it
 * ends: saved, cancelled with Esc, left by switching windows, or refused (D-061).
 */
function useRecorder(onDone: (message: string | null) => void) {
  const [recording, setRecording] = useState<ShortcutAction | null>(null);
  const [held, setHeld] = useState('');

  useEffect(() => {
    if (!recording) return;
    void suspendShortcuts(true);
    const finish = (message: string | null) => {
      setRecording(null);
      setHeld('');
      onDone(message);
    };
    const save = (accelerator: string | null) => {
      setShortcut(recording, accelerator).then(
        (next: Settings) => {
          useSettings.setState({ settings: next });
          finish(accelerator && clashesWithAltGr(accelerator) ? t('settings.shortcut.altGr') : null);
        },
        (err: unknown) => finish(String(err)),
      );
    };
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const plain = !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey;
      if (e.key === 'Escape' && plain) return finish(null);
      if ((e.key === 'Backspace' || e.key === 'Delete') && plain) return save(null);
      const accelerator = acceleratorFrom(e);
      if (accelerator) save(accelerator);
      else setHeld([e.metaKey && 'Win', e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift'].filter(Boolean).join('+'));
    };
    const onBlur = () => finish(null);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('blur', onBlur);
      void suspendShortcuts(false);
    };
  }, [recording, onDone]);

  return { recording, held, start: setRecording };
}

export function ShortcutsTab({ settings }: { settings: Settings }) {
  const update = useSettings((s) => s.update);
  const [message, setMessage] = useState<string | null>(null);
  const [unavailable, setUnavailable] = useState<ShortcutAction[]>([]);
  useEffect(() => {
    unavailableShortcuts().then(setUnavailable, () => {});
  }, [settings.shortcuts]);
  const { recording, held, start } = useRecorder(setMessage);

  return (
    <>
      <Section title={t('settings.shortcuts.global')}>
        {ACTIONS.map(([action, key, label]) => {
          const accelerator = settings.shortcuts[key];
          const listening = recording === action;
          return (
            <div key={action} className={styles.row}>
              <div className={styles.label}>
                <span>{t(label)}</span>
                {unavailable.includes(action) && accelerator && <small className={styles.warning}>{t('settings.shortcut.taken')}</small>}
                {listening && <small>{t('settings.shortcut.listening')}</small>}
              </div>
              <div className={styles.control}>
                {listening ? (
                  <kbd className={styles.listening} aria-live="polite">
                    {held || '…'}
                  </kbd>
                ) : accelerator ? (
                  <kbd>{formatShortcut(accelerator)}</kbd>
                ) : (
                  <span className={styles.muted}>{t('settings.shortcut.off')}</span>
                )}
                <button
                  type="button"
                  className={styles.button}
                  disabled={recording !== null && !listening}
                  onClick={() => {
                    setMessage(null);
                    start(listening ? null : action);
                  }}
                >
                  {t(listening ? 'settings.shortcut.cancel' : 'settings.shortcut.change')}
                </button>
              </div>
            </div>
          );
        })}
      </Section>
      {message && (
        <p className={styles.notice} role="status">
          {message}
        </p>
      )}
      <p className={styles.help}>{t('settings.shortcut.help')}</p>
      <button type="button" className={styles.button} disabled={recording !== null} onClick={() => void update({ shortcuts: DEFAULTS })}>
        {t('settings.shortcut.defaults')}
      </button>
    </>
  );
}
