import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useEffect, useState } from 'react';
import { t } from '../../i18n';
import { closeShelf, shelfHidden, type ShelfCloseEvent, type ShelfOpenEvent } from '../../ipc';
import { useSettings } from '../../state/settings';
import { EmptyState } from './EmptyState';
import styles from './Shelf.module.css';

// Matches the CSS transitions: open ~420 ms ease-out, close ~200 ms.
const SETTLE_MS = 430;
const CLOSE_MS = 210;

type Phase = 'hidden' | 'open' | 'settled' | 'closing';

const reducedMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

export function ShelfApp() {
  useSettings((s) => s.lang);
  const shelfShortcut = useSettings((s) => s.settings?.shortcuts.shelf ?? null);
  const [phase, setPhase] = useState<Phase>('hidden');

  useEffect(() => {
    const win = getCurrentWebviewWindow();
    let settleTimer: number | undefined;
    const opened = win.listen<ShelfOpenEvent>('shelf:open', () => {
      window.clearTimeout(settleTimer);
      setPhase('open');
      // Keys work at once; the mouse waits until the board has landed.
      settleTimer = window.setTimeout(() => setPhase('settled'), reducedMotion() ? 0 : SETTLE_MS);
    });
    const closed = win.listen<ShelfCloseEvent>('shelf:close', ({ payload }) => {
      window.clearTimeout(settleTimer);
      setPhase('closing');
      // Rust hides the window only if no newer open happened in the meantime.
      window.setTimeout(() => void shelfHidden(payload.generation), reducedMotion() ? 0 : CLOSE_MS);
    });
    return () => {
      window.clearTimeout(settleTimer);
      void opened.then((unlisten) => unlisten());
      void closed.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        void closeShelf();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  const isOpen = phase === 'open' || phase === 'settled';
  const wrapClass = [styles.boardWrap, isOpen && styles.open, phase === 'settled' && styles.settled]
    .filter(Boolean)
    .join(' ');

  return (
    <div className={styles.layer}>
      <div className={styles.backdrop} onClick={() => void closeShelf()} />
      <div className={wrapClass}>
        <section className={styles.board} aria-label={t('shelf.label')}>
          <div className={`${styles.frame} ${styles.frameTop}`} />
          <div className={`${styles.frame} ${styles.frameLeft}`} />
          <div className={`${styles.frame} ${styles.frameRight}`} />
          <div className={`${styles.frame} ${styles.frameBottom}`}>
            <span className={styles.engrave}>{t('shelf.hints')}</span>
          </div>
          <div className={styles.cork}>
            <EmptyState shelfShortcut={shelfShortcut} />
          </div>
        </section>
      </div>
    </div>
  );
}
