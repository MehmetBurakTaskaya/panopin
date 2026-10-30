import { useCallback, useEffect, useRef, useState } from 'react';
import { formatLocale, t } from '../../../i18n';
import { getText, updateText, type ShelfItem } from '../../../ipc';
import { icons } from './icons';
import type { PreviewMode } from './Preview';
import styles from './Preview.module.css';
import { trackSave } from './saves';

/** Typing pauses this long before the text is saved. */
export const AUTOSAVE_MS = 300;

export interface TextPreviewProps {
  item: ShelfItem;
  editing: boolean;
  onMode: (mode: PreviewMode) => void;
  onClose: () => void;
}

/** The whole text on a sheet. E or a click edits it in place; it saves about 300 ms after typing
 * stops, and Ctrl+Enter or Esc saves and finishes. */
export function TextPreview({ item, editing, onMode, onClose }: TextPreviewProps) {
  const [body, setBody] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const savedBody = useRef<string | null>(null);
  const latest = useRef<string | null>(null);
  const id = item.id;

  useEffect(() => {
    let alive = true;
    getText(id)
      .then((text) => {
        if (!alive) return;
        savedBody.current = text;
        latest.current = text;
        setBody(text);
      })
      .catch((err: unknown) => {
        if (alive) setError(String(err));
      });
    return () => {
      alive = false;
    };
  }, [id]);

  const flush = useCallback(() => {
    window.clearTimeout(timer.current);
    const text = latest.current;
    if (text === null || text === savedBody.current) return;
    savedBody.current = text;
    trackSave(updateText(id, text)).catch((err: unknown) => setError(String(err)));
  }, [id]);

  // Moving to another card or closing the preview saves first.
  useEffect(() => () => flush(), [flush]);

  const change = (text: string) => {
    latest.current = text;
    setBody(text);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(flush, AUTOSAVE_MS);
  };

  const finish = () => {
    flush();
    onMode('view');
  };

  const count = body === null ? 0 : [...body].length;
  return (
    <>
      <div className={styles.bar}>
        <span className={styles.title}>{editing ? t('preview.editing') : t('preview.text')}</span>
        {editing ? (
          <button type="button" className={`${styles.button} ${styles.primary}`} onClick={finish}>
            {icons.check}
            {t('markup.done')}
          </button>
        ) : (
          <button type="button" className={styles.button} title={`${t('preview.edit')} (E)`} onClick={() => onMode('text')} disabled={body === null}>
            {icons.pen}
            {t('preview.edit')}
          </button>
        )}
        <span className={styles.sep} />
        <button type="button" className={styles.button} title={t('preview.close')} aria-label={t('preview.close')} onClick={onClose}>
          {icons.close}
        </button>
      </div>
      <div className={styles.body}>
        <div className={styles.sheet}>
          {editing && body !== null ? (
            <textarea
              className={styles.sheetEdit}
              dir="auto"
              value={body}
              autoFocus
              spellCheck={false}
              aria-label={t('preview.edit')}
              onChange={(e) => change(e.target.value)}
              onKeyDown={(e) => {
                // Typing never reaches the shelf shortcuts.
                e.stopPropagation();
                if (e.key === 'Escape' || (e.key === 'Enter' && (e.ctrlKey || e.metaKey))) {
                  e.preventDefault();
                  finish();
                }
              }}
            />
          ) : (
            <pre className={styles.sheetText} dir="auto" onClick={() => body !== null && onMode('text')}>
              {body ?? ''}
            </pre>
          )}
          <div className={styles.sheetMeta}>
            <span>{count === 1 ? t('shelf.card.charsOne') : t('shelf.card.chars', { count: new Intl.NumberFormat(formatLocale()).format(count) })}</span>
            <span>{error ? t('markup.saveFailed', { error }) : editing ? t('preview.autosave') : t('preview.editHint')}</span>
          </div>
        </div>
      </div>
    </>
  );
}
