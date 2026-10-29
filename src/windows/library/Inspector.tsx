import { useCallback, useEffect, useLayoutEffect, useRef, useState, type Ref } from 'react';
import { folderPaths } from '../../components/pickers/Pickers';
import { getLanguage, t, type MessageKey } from '../../i18n';
import { removeFromFolder, setNote, setTitle, untagItems, type Folder, type LibraryItem, type Tag } from '../../ipc';
import { duration } from '../shelf/paper';
import styles from './Library.module.css';

/** Inspector edits save this long after typing stops. */
const SAVE_MS = 500;

const SOURCE: Record<LibraryItem['source'], MessageKey> = {
  clipboard: 'library.source.clipboard',
  folder: 'library.source.folder',
  import: 'library.source.import',
  manual: 'library.source.manual',
};

/** Keeps a draft, saves it after a pause and keeps it (with the focus) if saving fails. */
function useAutosave(value: string, key: number, save: (value: string) => Promise<void>) {
  const [draft, setDraft] = useState(value);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const pending = useRef<string | null>(null);
  const saving = useRef(0);
  const saver = useRef(save);
  const stored = useRef(value);
  useLayoutEffect(() => {
    saver.current = save;
    stored.current = value;
  });

  const flush = useCallback(() => {
    window.clearTimeout(timer.current);
    const text = pending.current;
    pending.current = null;
    if (text === null) return;
    saving.current++;
    saver.current(text)
      .then(
        () => setError(null),
        (err: unknown) => setError(String(err)),
      )
      .finally(() => saving.current--);
  }, []);

  // Another item starts from its stored value; anything still pending is saved first.
  useEffect(() => {
    setDraft(stored.current);
    setError(null);
    return flush;
  }, [key, flush]);

  // A change made elsewhere (a picker, the other window) shows up unless the user is typing.
  useEffect(() => {
    if (pending.current === null && saving.current === 0) setDraft(value);
  }, [value]);

  const change = (text: string) => {
    setDraft(text);
    pending.current = text;
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(flush, SAVE_MS);
  };
  return { draft, change, flush, error };
}

export interface InspectorProps {
  paneRef: Ref<HTMLDivElement>;
  items: LibraryItem[];
  folders: Folder[];
  tags: Map<number, Tag>;
  inTrash: boolean;
  onName: () => void;
  onFolder: () => void;
  onTag: () => void;
  onPin: () => void;
  onExport: () => void;
  onTrash: () => void;
  onRestore: () => void;
  onDeleteForever: () => void;
  onOpenLink: (id: number) => void;
  onRefreshLink: (id: number) => void;
}

function Single({ item, folders, tags }: { item: LibraryItem; folders: Folder[]; tags: Map<number, Tag> }) {
  const title = useAutosave(item.title ?? '', item.id, (text) => setTitle([item.id], text.trim() ? text : null));
  const note = useAutosave(item.note ?? '', item.id, (text) => setNote(item.id, text.trim() ? text : null));
  const date = new Intl.DateTimeFormat(getLanguage(), { dateStyle: 'medium', timeStyle: 'short' });
  const paths = new Map(folderPaths(folders).map((f) => [f.id, f.path]));
  const size =
    item.kind === 'image' && item.width && item.height
      ? `${item.width}×${item.height}`
      : item.charCount !== null
        ? t('shelf.card.chars', { count: new Intl.NumberFormat(getLanguage()).format(item.charCount) })
        : null;
  const source = [t(SOURCE[item.source]), item.sourceApp, item.sourcePath].filter(Boolean).join(' · ');

  return (
    <>
      <label className={styles.field}>
        <span>{t('library.inspector.title')}</span>
        <input value={title.draft} placeholder={t('library.inspector.untitled')} onChange={(e) => title.change(e.target.value)} onBlur={title.flush} />
      </label>
      {title.error && <span className={styles.saveError}>{t('markup.saveFailed', { error: title.error })}</span>}
      <label className={styles.field}>
        <span>{t('library.inspector.note')}</span>
        <textarea value={note.draft} onChange={(e) => note.change(e.target.value)} onBlur={note.flush} />
      </label>
      {note.error && <span className={styles.saveError}>{t('markup.saveFailed', { error: note.error })}</span>}
      <div className={styles.field}>
        <span>{t('library.inspector.tags')}</span>
        <div className={styles.chips}>
          {item.tags.map((id) => (
            <span key={id} className={styles.chip}>
              # {tags.get(id)?.name}
              <button type="button" aria-label={t('library.inspector.removeTag', { name: tags.get(id)?.name ?? '' })} onClick={() => void untagItems([item.id], id)}>
                ×
              </button>
            </span>
          ))}
        </div>
      </div>
      <div className={styles.field}>
        <span>{t('library.inspector.folders')}</span>
        <div className={styles.chips}>
          {item.folders.map((id) => (
            <span key={id} className={styles.chip}>
              {paths.get(id)}
              <button type="button" aria-label={t('library.inspector.removeFolder', { name: paths.get(id) ?? '' })} onClick={() => void removeFromFolder([item.id], id)}>
                ×
              </button>
            </span>
          ))}
        </div>
      </div>
      <dl className={styles.facts}>
        {item.kind === 'link' && (
          <>
            <dt>{t('library.inspector.address')}</dt>
            <dd>{item.url}</dd>
            {item.linkTitle && (
              <>
                <dt>{t('library.inspector.page')}</dt>
                <dd>{[item.linkTitle, item.linkSite].filter(Boolean).join(' · ')}</dd>
              </>
            )}
          </>
        )}
        {item.kind === 'video' && item.durationMs !== null && (
          <>
            <dt>{t('library.inspector.length')}</dt>
            <dd>{duration(item.durationMs)}</dd>
          </>
        )}
        <dt>{t('library.inspector.source')}</dt>
        <dd>{source}</dd>
        {size && (
          <>
            <dt>{t('library.inspector.size')}</dt>
            <dd>{size}</dd>
          </>
        )}
        <dt>{t('library.inspector.added')}</dt>
        <dd>{date.format(item.createdAt)}</dd>
        <dt>{t('library.inspector.copied')}</dt>
        <dd>{date.format(item.lastCopiedAt)}</dd>
        {item.trashedAt !== null && (
          <>
            <dt>{t('library.inspector.trashed')}</dt>
            <dd>{date.format(item.trashedAt)}</dd>
          </>
        )}
      </dl>
    </>
  );
}

/** Details of the selection. Changes save on their own; a failed save keeps what was typed. */
export function Inspector(props: InspectorProps) {
  const { items, inTrash } = props;
  const [first] = items;
  const allPinned = items.length > 0 && items.every((item) => item.pinned);
  return (
    <div ref={props.paneRef} className={`${styles.pane} ${styles.inspector}`} tabIndex={-1} aria-label={t('library.inspector')}>
      {items.length === 0 && <span className={styles.empty}>{t('library.inspector.none')}</span>}
      {items.length > 1 && <h2>{t('library.inspector.many', { count: items.length })}</h2>}
      {items.length === 1 && first && <Single key={first.id} item={first} folders={props.folders} tags={props.tags} />}
      {items.length > 0 && (
        <div className={styles.actions}>
          {inTrash ? (
            <>
              <button type="button" className={styles.tool} onClick={props.onRestore}>
                {t('library.action.restore')} (R)
              </button>
              <button type="button" className={styles.tool} onClick={props.onDeleteForever}>
                {t('library.action.deleteForever')} (Delete)
              </button>
            </>
          ) : (
            <>
              {items.length === 1 && first?.kind === 'link' && (
                <>
                  <button type="button" className={styles.tool} onClick={() => props.onOpenLink(first.id)}>
                    {t('shelf.menu.openLink')} (Ctrl+O)
                  </button>
                  <button type="button" className={styles.tool} onClick={() => props.onRefreshLink(first.id)}>
                    {t('shelf.menu.refreshLink')}
                  </button>
                </>
              )}
              <button type="button" className={styles.tool} onClick={props.onPin}>
                {t(allPinned ? 'shelf.menu.unpin' : 'shelf.menu.pin')} (P)
              </button>
              <button type="button" className={styles.tool} onClick={props.onName}>
                {t('library.action.name')} (Alt+P)
              </button>
              <button type="button" className={styles.tool} onClick={props.onFolder}>
                {t('library.action.folder')} (Shift+P)
              </button>
              <button type="button" className={styles.tool} onClick={props.onTag}>
                {t('library.action.tag')} (Ctrl+Shift+T)
              </button>
              <button type="button" className={styles.tool} onClick={props.onExport}>
                {t('library.action.export')} (Ctrl+Shift+E)
              </button>
              <button type="button" className={styles.tool} onClick={props.onTrash}>
                {t('shelf.menu.trash')}
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
