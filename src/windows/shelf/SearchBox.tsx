import type { Ref } from 'react';
import { KeyText } from '../../components/Keys';
import { PinIcon } from '../../components/PinIcon';
import { t, type MessageKey } from '../../i18n';
import { KINDS, type Kind } from '../../keyboard/shelfModel';
import styles from './SearchBox.module.css';

const LABEL: Record<Kind, MessageKey> = { image: 'shelf.row.image', text: 'shelf.row.text', link: 'shelf.row.link' };

export interface SearchBoxProps {
  inputRef: Ref<HTMLInputElement>;
  query: string;
  /** Matches per visible row; `null` while nothing searchable is typed. */
  counts: Record<Kind, number> | null;
  hidden: Record<Kind, boolean>;
  withoutAccents: boolean;
  /** Distance of the box's centre from the top of the screen. */
  top: number;
  onQuery: (query: string) => void;
  onAccents: () => void;
  /** Enter, ↑ or ↓: on to the results. */
  onResults: () => void;
  /** Esc: clears and closes. */
  onClose: () => void;
}

/**
 * Shift+S on the shelf: a note pinned under the board, with the rows showing the matches. It
 * counts them per row and names the keys that leave it.
 */
export function SearchBox(props: SearchBoxProps) {
  const { counts, hidden } = props;
  const total = counts ? KINDS.reduce((sum, kind) => sum + (hidden[kind] ? 0 : counts[kind]), 0) : 0;
  return (
    <div className={styles.note} style={{ top: props.top }} role="search">
      <PinIcon className={styles.pin} />
      <label className={styles.field}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="2" />
          <path d="M15.5 15.5 21 21" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <input
          ref={props.inputRef}
          // The box exists only while searching, so it takes the focus as it appears: a focus
          // request made before React has added it would leave typing to the shelf's letter keys.
          autoFocus
          type="search"
          value={props.query}
          placeholder={t('shelf.search.placeholder')}
          aria-label={t('shelf.search.label')}
          aria-describedby="shelf-search-count"
          autoComplete="off"
          spellCheck={false}
          onChange={(e) => props.onQuery(e.target.value)}
          onKeyDown={(e) => {
            // Typing never reaches the shelf's letter shortcuts.
            e.stopPropagation();
            if (e.key === 'Escape') {
              e.preventDefault();
              props.onClose();
            } else if (e.key === 'Enter' || e.key === 'ArrowDown' || e.key === 'ArrowUp') {
              e.preventDefault();
              props.onResults();
            }
          }}
        />
        <span id="shelf-search-count" className={styles.count} aria-live="polite">
          {counts ? (total === 1 ? t('shelf.search.countOne') : t('shelf.search.count', { count: total })) : ''}
        </span>
      </label>
      <div className={styles.foot}>
        {counts &&
          KINDS.filter((kind) => !hidden[kind]).map((kind) => (
            <span key={kind} className={`${styles.tag} ${counts[kind] === 0 ? styles.zero : ''}`} data-kind={kind}>
              {t(LABEL[kind])}
              <b>{counts[kind]}</b>
            </span>
          ))}
        <button type="button" className={styles.accents} aria-pressed={props.withoutAccents} onClick={props.onAccents}>
          <i aria-hidden="true" />
          {t('shelf.search.accents')}
        </button>
      </div>
      <span className={styles.hint}>
        <KeyText text={t('shelf.search.hint')} keys={{ enter: 'Enter', esc: 'Esc' }} />
      </span>
    </div>
  );
}
