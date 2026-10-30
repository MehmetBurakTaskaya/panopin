import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { formatLocale, t } from '../../i18n';
import { checkForUpdates, installUpdate, updateStatus, type UpdateView } from '../../ipc';
import { Section } from './controls';
import styles from './Settings.module.css';

/** Release notes are Markdown; they are shown as plain text, never as HTML. Pure, so it is tested. */
export function plainNotes(markdown: string): string {
  return markdown
    .replace(/\r\n?/g, '\n')
    // A line that starts no heading, list item or paragraph continues the one above.
    .replace(/([^\n])\n(?!\n|#|[ \t]*[-*][ \t])[ \t]*/g, '$1 ')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/\*\*|__|`/g, '')
    .replace(/^#+[ \t]*/gm, '')
    .replace(/^[ \t]*[-*][ \t]+/gm, '• ')
    .trim();
}

const when = (ms: number) => new Date(ms).toLocaleString(formatLocale(), { dateStyle: 'medium', timeStyle: 'short' });

function statusLine(view: UpdateView): string {
  const status = view.status;
  switch (status.state) {
    case 'idle':
      return t('settings.updates.current', { version: view.current });
    case 'checking':
      return t('settings.updates.checking');
    case 'upToDate':
      return `${t('settings.updates.upToDate', { version: view.current })} ${t('settings.updates.checkedAt', { time: when(status.checkedAt) })}`;
    case 'available':
      // An update installs by itself once Panopin is not in use, unless that would ask for a password (D-076).
      return t(view.automatic ? 'settings.updates.pending' : 'settings.updates.available', { version: status.version });
    case 'downloading':
      return status.total
        ? t('settings.updates.downloading', { percent: Math.min(100, Math.round((status.received / status.total) * 100)) })
        : t('settings.updates.downloadingNoSize');
    case 'failed':
      return t('settings.updates.failed');
  }
}

/** Settings → Updates (D-065, D-076): updates are always looked for; "Check now" and "Update and restart". */
export function UpdatesTab() {
  const [view, setView] = useState<UpdateView | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    updateStatus().then(setView, () => {});
    const off = listen<UpdateView>('updates:changed', (event) => setView(event.payload));
    return () => void off.then((stop) => stop(), () => {});
  }, []);
  const status = view?.status;
  const busy = status?.state === 'checking' || status?.state === 'downloading';
  const install = () => {
    setFailed(false);
    installUpdate().catch(() => setFailed(true));
  };

  return (
    <Section title={t('settings.updates.section')}>
      <p className={styles.help}>{t('settings.updates.automatic')}</p>
      <div className={styles.row}>
        <span className={styles.label} role="status" aria-live="polite">
          {view && statusLine(view)}
        </span>
        <button
          type="button"
          className={styles.button}
          disabled={busy}
          onClick={() => {
            setFailed(false);
            checkForUpdates().then(setView, () => {});
          }}
        >
          {t('settings.updates.checkNow')}
        </button>
      </div>
      {status?.state === 'downloading' && (
        <progress className={styles.progress} max={status.total ?? undefined} value={status.total ? status.received : undefined} />
      )}
      {status?.state === 'available' && view && (
        <div className={styles.offer}>
          {status.notes && (
            <>
              <h3>{t('settings.updates.notes')}</h3>
              <p className={styles.notes}>{plainNotes(status.notes)}</p>
            </>
          )}
          {view.installable ? (
            <p className={styles.help}>{t('settings.updates.installHelp')}</p>
          ) : (
            <p className={styles.help}>{t('settings.updates.manual')}</p>
          )}
          <button type="button" className={`${styles.button} ${styles.primary}`} onClick={install}>
            {t(view.installable ? 'settings.updates.install' : 'settings.updates.download')}
          </button>
        </div>
      )}
      {failed && (
        <p className={styles.error} role="alert">
          {t('settings.updates.installFailed')}
        </p>
      )}
    </Section>
  );
}
