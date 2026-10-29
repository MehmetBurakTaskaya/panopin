import { useId, useMemo, useState, type KeyboardEvent, type ReactNode } from 'react';
import { getLanguage, t } from '../../i18n';
import type { Folder, Tag } from '../../ipc';
import styles from './Pickers.module.css';

/** Matches names the way Rust keys tags: the four Turkish i forms are one letter, case does not
 * count, accents do. */
const fold = (text: string) => text.replace(/[İIı]/g, 'i').toLocaleLowerCase(getLanguage());

function Dialog({ title, detail, children, onClose }: { title: string; detail?: string; children: ReactNode; onClose: () => void }) {
  const id = useId();
  return (
    <div
      className={styles.scrim}
      onPointerDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby={id}>
        <div className={styles.head}>
          <span id={id}>{title}</span>
          {detail && <small>{detail}</small>}
        </div>
        {children}
      </div>
    </div>
  );
}

interface ListKeys {
  count: number;
  active: number;
  setActive: (index: number) => void;
  onEnter: () => void;
  onClose: () => void;
  extra?: (e: KeyboardEvent<HTMLInputElement>) => boolean;
}

function listKeys({ count, active, setActive, onEnter, onClose, extra }: ListKeys) {
  return (e: KeyboardEvent<HTMLInputElement>) => {
    // Typing in the box never reaches the window shortcuts behind it.
    e.stopPropagation();
    if (extra?.(e)) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (count > 0) setActive((active + (e.key === 'ArrowDown' ? 1 : count - 1)) % count);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      onEnter();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  };
}

/** Folders shown with their parents: "Work › Invoices". */
export function folderPaths(folders: Folder[]): { id: number; path: string; name: string }[] {
  const byId = new Map(folders.map((f) => [f.id, f]));
  const pathOf = (folder: Folder): string => {
    const names = [folder.name];
    let parent = folder.parentId === null ? undefined : byId.get(folder.parentId);
    for (let depth = 0; parent && depth < 32; depth++) {
      names.unshift(parent.name);
      parent = parent.parentId === null ? undefined : byId.get(parent.parentId);
    }
    return names.join(' › ');
  };
  return folders
    .map((f) => ({ id: f.id, name: f.name, path: pathOf(f) }))
    .sort((a, b) => a.path.localeCompare(b.path, getLanguage(), { sensitivity: 'base' }));
}

/** Shift+P: a searchable folder list. Enter adds to the folder (and pins); Ctrl+N makes a new
 * folder from what was typed. */
export function FolderPicker({
  folders,
  onPick,
  onCreate,
  onClose,
}: {
  folders: Folder[];
  onPick: (folder: number) => void;
  onCreate: (name: string) => Promise<number>;
  onClose: () => void;
}) {
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const shown = useMemo(() => folderPaths(folders).filter((f) => fold(f.path).includes(fold(query.trim()))), [folders, query]);

  const create = () => {
    const name = query.trim() || t('library.folder.new');
    onCreate(name).then(onPick, (err: unknown) => setError(String(err)));
  };

  return (
    <Dialog title={t('picker.folder.title')} onClose={onClose}>
      <input
        className={styles.input}
        autoFocus
        value={query}
        placeholder={t('picker.folder.search')}
        aria-label={t('picker.folder.search')}
        onChange={(e) => {
          setQuery(e.target.value);
          setActive(0);
        }}
        onKeyDown={listKeys({
          count: shown.length,
          active,
          setActive,
          onClose,
          onEnter: () => {
            const choice = shown[active];
            if (choice) onPick(choice.id);
            else if (query.trim()) create();
          },
          extra: (e) => {
            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
              e.preventDefault();
              create();
              return true;
            }
            return false;
          },
        })}
      />
      <ul className={styles.list} role="listbox" aria-label={t('picker.folder.title')}>
        {shown.map((folder, i) => (
          <li
            key={folder.id}
            className={styles.option}
            role="option"
            aria-selected={i === active}
            onMouseEnter={() => setActive(i)}
            onClick={() => onPick(folder.id)}
          >
            {folder.path}
          </li>
        ))}
        {shown.length === 0 && <li className={`${styles.option} ${styles.create}`}>{t('picker.folder.none')}</li>}
      </ul>
      {error && <div className={styles.error}>{error}</div>}
      <div className={styles.foot}>
        <span>{t('picker.folder.hint')}</span>
        <span>{t('picker.pins')}</span>
      </div>
    </Dialog>
  );
}

/** Ctrl+Shift+T: pick an existing tag or create one; the items keep their other tags. */
export function TagPicker({ tags, onPick, onClose }: { tags: Tag[]; onPick: (name: string) => void; onClose: () => void }) {
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const typed = query.trim();
  const shown = useMemo(() => tags.filter((tag) => fold(tag.name).includes(fold(typed))), [tags, typed]);
  const exact = shown.some((tag) => fold(tag.name) === fold(typed));
  const options = [...shown.map((tag) => tag.name), ...(typed && !exact ? [typed] : [])];

  return (
    <Dialog title={t('picker.tag.title')} onClose={onClose}>
      <input
        className={styles.input}
        autoFocus
        value={query}
        placeholder={t('picker.tag.search')}
        aria-label={t('picker.tag.search')}
        onChange={(e) => {
          setQuery(e.target.value);
          setActive(0);
        }}
        onKeyDown={listKeys({
          count: options.length,
          active,
          setActive,
          onClose,
          onEnter: () => {
            const choice = options[active];
            if (choice) onPick(choice);
          },
        })}
      />
      <ul className={styles.list} role="listbox" aria-label={t('picker.tag.title')}>
        {options.map((name, i) => (
          <li
            key={`${name}-${i}`}
            className={`${styles.option} ${i >= shown.length ? styles.create : ''}`}
            role="option"
            aria-selected={i === active}
            onMouseEnter={() => setActive(i)}
            onClick={() => onPick(name)}
          >
            {i >= shown.length ? t('picker.tag.create', { name }) : `# ${name}`}
          </li>
        ))}
        {options.length === 0 && <li className={`${styles.option} ${styles.create}`}>{t('picker.tag.none')}</li>}
      </ul>
      <div className={styles.foot}>
        <span>{t('picker.tag.hint')}</span>
      </div>
    </Dialog>
  );
}

/** A yes/no question with the safe answer one key away: Enter confirms, Esc cancels. */
export function ConfirmDialog({
  title,
  body,
  confirm,
  onConfirm,
  onClose,
}: {
  title: string;
  body?: string;
  confirm: string;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <Dialog title={title} onClose={onClose}>
      {body && <p className={styles.foot}>{body}</p>}
      <div className={styles.foot}>
        <span />
        <span>
          <button
            type="button"
            className={styles.button}
            onClick={onClose}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === 'Escape') onClose();
            }}
          >
            {t('shelf.trash.cancel')}
          </button>{' '}
          <button
            type="button"
            className={`${styles.button} ${styles.danger}`}
            autoFocus
            onClick={onConfirm}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === 'Escape') onClose();
            }}
          >
            {confirm}
          </button>
        </span>
      </div>
    </Dialog>
  );
}

export interface Nameable {
  id: number;
  title: string | null;
  label: string;
}

/** Alt+P: names the items one after another (and pins them). Enter saves and moves on; Esc
 * leaves the rest as they are. */
export function NamePrompt({ items, onSave, onClose }: { items: Nameable[]; onSave: (id: number, title: string) => Promise<void>; onClose: () => void }) {
  const [index, setIndex] = useState(0);
  const [value, setValue] = useState(items[0]?.title ?? '');
  const [error, setError] = useState<string | null>(null);
  const current = items[index];
  if (!current) return null;

  const save = () => {
    onSave(current.id, value).then(
      () => {
        const next = items[index + 1];
        if (!next) {
          onClose();
          return;
        }
        setIndex(index + 1);
        setValue(next.title ?? '');
        setError(null);
      },
      // The draft and the focus stay, so nothing typed is lost.
      (err: unknown) => setError(String(err)),
    );
  };

  return (
    <Dialog title={t('picker.name.title')} detail={items.length > 1 ? t('picker.name.progress', { n: index + 1, total: items.length }) : current.label} onClose={onClose}>
      <input
        key={current.id}
        className={styles.input}
        autoFocus
        value={value}
        placeholder={current.label}
        aria-label={t('picker.name.title')}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === 'Enter') {
            e.preventDefault();
            save();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            onClose();
          }
        }}
      />
      {error && <div className={styles.error}>{error}</div>}
      <div className={styles.foot}>
        <span>{items.length > 1 ? current.label : t('picker.name.hint')}</span>
        <span>{t('picker.pins')}</span>
      </div>
    </Dialog>
  );
}
