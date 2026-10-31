import { useEffect } from 'react';
import { Keys, KeyText } from '../../components/Keys';
import { PinIcon } from '../../components/PinIcon';
import { t } from '../../i18n';
import type { KeyGroup } from '../../keyboard/keyList';
import styles from './KeyNote.module.css';

/** ? or F1 on the shelf: every key on a note pinned over the screen. Esc, ? or F1 takes it down. */
export function KeyNote({ groups, onClose }: { groups: KeyGroup[]; onClose: () => void }) {
  useEffect(() => {
    // Before the shelf's own keys, which stay quiet while the note is up.
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' && event.key !== '?' && event.key !== 'F1') return;
      event.preventDefault();
      event.stopPropagation();
      onClose();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [onClose]);

  return (
    <div className={styles.scrim} onClick={onClose}>
      <section
        className={styles.note}
        role="dialog"
        aria-modal="true"
        aria-labelledby="key-note-title"
        onClick={(event) => event.stopPropagation()}
      >
        <PinIcon className={styles.pin} />
        <h2 id="key-note-title">{t('keys.title')}</h2>
        <div className={styles.groups}>
          {groups.map((group) => (
            <section key={group.title} className={styles.group}>
              <h3>{t(group.title)}</h3>
              <dl>
                {group.entries.map((entry) => (
                  <div key={`${entry.label} ${entry.keys.join(' ')}`} className={styles.entry}>
                    <dt>{t(entry.label)}</dt>
                    <dd>
                      <Keys keys={entry.keys} className={styles.keys} />
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
        </div>
        <p className={styles.foot}>
          <KeyText text={t('keys.footer')} keys={{ esc: 'Esc', palette: 'CommandOrControl+K' }} />
        </p>
      </section>
    </div>
  );
}
