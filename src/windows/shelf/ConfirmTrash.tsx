import { useEffect, useId, useRef, type KeyboardEvent } from 'react';
import { PinIcon } from '../../components/PinIcon';
import { t } from '../../i18n';
import styles from './Shelf.module.css';

/** Delete asks first: a note pinned over the board with the two choices. */
export function ConfirmTrash({ count, onConfirm, onCancel }: { count: number; onConfirm: () => void; onCancel: () => void }) {
  const titleId = useId();
  const confirm = useRef<HTMLButtonElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);

  useEffect(() => confirm.current?.focus(), []);

  const onKeyDown = (event: KeyboardEvent) => {
    event.stopPropagation();
    if (event.key === 'Escape') {
      event.preventDefault();
      onCancel();
    } else if (event.key === 'Tab' || event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      // Two buttons: every move switches between them and focus never leaves the note.
      event.preventDefault();
      (document.activeElement === confirm.current ? cancel : confirm).current?.focus();
    }
  };

  return (
    <div className={styles.scrim} onPointerDown={(event) => event.target === event.currentTarget && onCancel()}>
      <div className={styles.dialog} role="alertdialog" aria-modal="true" aria-labelledby={titleId} onKeyDown={onKeyDown}>
        <PinIcon className={styles.notePin} />
        <h2 id={titleId}>{count === 1 ? t('shelf.trash.confirmOne') : t('shelf.trash.confirm', { count })}</h2>
        <div className={styles.dialogButtons}>
          <button ref={cancel} type="button" onClick={onCancel}>
            {t('shelf.trash.cancel')}
          </button>
          <button ref={confirm} type="button" className={styles.danger} onClick={onConfirm}>
            {t('shelf.trash.ok')}
          </button>
        </div>
      </div>
    </div>
  );
}
