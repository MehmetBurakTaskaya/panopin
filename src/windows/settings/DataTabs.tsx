import { useState } from 'react';
import { t, type MessageKey } from '../../i18n';
import { clearHistory, countClearable, pickWatchFolder, type HistoryKind, type Retention, type Settings, type Sources } from '../../ipc';
import { useSettings } from '../../state/settings';
import { Choice, Section, Toggle } from './controls';
import styles from './Settings.module.css';

const SOURCES: [keyof Sources, MessageKey, MessageKey | null][] = [
  ['screenshots', 'settings.source.screenshots', 'settings.source.screenshotsHint'],
  ['clipboardImages', 'settings.source.clipboardImages', null],
  ['clipboardText', 'settings.source.clipboardText', 'settings.source.clipboardTextHint'],
  ['links', 'settings.source.links', null],
  ['screenRecordings', 'settings.source.screenRecordings', 'settings.source.screenRecordingsHint'],
];

const MAX_FOLDERS = 3;

export function SourcesTab({ settings }: { settings: Settings }) {
  const update = useSettings((s) => s.update);
  const folders = settings.watchedFolders;
  const addFolder = () => {
    pickWatchFolder().then((folder) => {
      if (folder && !folders.includes(folder)) void update({ watchedFolders: [...folders, folder] });
    }, () => {});
  };
  return (
    <>
      <Section title={t('settings.sources.what')}>
        {SOURCES.map(([key, label, hint]) => (
          <Toggle
            key={key}
            label={t(label)}
            hint={hint ? t(hint) : undefined}
            checked={settings.sources[key]}
            onChange={(on) => void update({ sources: { [key]: on } })}
          />
        ))}
      </Section>
      <Section title={t('settings.sources.folders')}>
        <p className={styles.help}>{t('settings.sources.foldersHelp')}</p>
        {folders.map((folder) => (
          <div key={folder} className={styles.row}>
            <span className={styles.path}>{folder}</span>
            <button
              type="button"
              className={styles.button}
              onClick={() => void update({ watchedFolders: folders.filter((f) => f !== folder) })}
            >
              {t('settings.sources.remove')}
            </button>
          </div>
        ))}
        <div className={styles.buttons}>
          <button type="button" className={styles.button} disabled={folders.length >= MAX_FOLDERS} onClick={addFolder}>
            {t('settings.sources.add')}
          </button>
        </div>
      </Section>
    </>
  );
}

const KINDS: [keyof Retention, MessageKey][] = [
  ['image', 'settings.history.image'],
  ['text', 'settings.history.text'],
  ['link', 'settings.history.link'],
  ['video', 'settings.history.video'],
];

const DAYS = [1, 3, 7, 15, 30];

const CLEAR: [string, MessageKey, HistoryKind[]][] = [
  ['all', 'settings.history.clearAll', ['image', 'text', 'link', 'video']],
  ['image', 'settings.history.clearImages', ['image', 'video']],
  ['text', 'settings.history.clearTexts', ['text']],
  ['link', 'settings.history.clearLinks', ['link']],
];

/** Retention per kind, Trash, and "Clear history": count first, confirm, then one transaction. */
export function HistoryTab({ settings }: { settings: Settings }) {
  const update = useSettings((s) => s.update);
  const [which, setWhich] = useState('all');
  const [pending, setPending] = useState<{ count: number; cutoff: number } | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const kinds = CLEAR.find(([key]) => key === which)?.[2] ?? [];
  const keep = (days: number | null) => (days === null ? 'forever' : String(days));
  const days = (value: string) => (value === 'forever' ? null : Number(value));
  const dayOptions: [string, string][] = [
    ...DAYS.map((n): [string, string] => [String(n), n === 1 ? t('settings.history.dayOne') : t('settings.history.days', { count: n })]),
    ['forever', t('settings.history.forever')],
  ];

  return (
    <>
      <Section title={t('settings.history.keep')}>
        <p className={styles.help}>{t('settings.history.keepHelp')}</p>
        {KINDS.map(([kind, label]) => (
          <Choice
            key={kind}
            label={t(label)}
            value={keep(settings.retention[kind])}
            options={dayOptions}
            onChange={(value) => void update({ retention: { [kind]: days(value) } })}
          />
        ))}
        <Choice
          label={t('settings.history.trash')}
          value={keep(settings.retention.trash)}
          options={[
            ['7', t('settings.history.trashAfter', { count: 7 })],
            ['30', t('settings.history.trashAfter', { count: 30 })],
            ['forever', t('settings.history.trashNever')],
          ]}
          onChange={(value) => void update({ retention: { trash: days(value) } })}
        />
      </Section>
      <Section title={t('settings.history.clear')}>
        <p className={styles.help}>{t('settings.history.clearHelp')}</p>
        <div className={styles.row} role="radiogroup" aria-label={t('settings.history.clear')}>
          {CLEAR.map(([key, label]) => (
            <label key={key} className={styles.radio}>
              <input
                type="radio"
                name="clear"
                checked={which === key}
                onChange={() => {
                  setWhich(key);
                  setPending(null);
                }}
              />
              {t(label)}
            </label>
          ))}
        </div>
        {pending ? (
          <div className={styles.confirm} role="alertdialog" aria-label={t('settings.history.clear')}>
            <span>
              {pending.count === 0 ? t('settings.history.nothing') : t('settings.history.confirm', { count: pending.count })}
            </span>
            <span className={styles.buttons}>
              <button type="button" className={styles.button} onClick={() => setPending(null)}>
                {t('shelf.trash.cancel')}
              </button>
              {pending.count > 0 && (
                <button
                  type="button"
                  className={`${styles.button} ${styles.danger}`}
                  autoFocus
                  onClick={() => {
                    clearHistory(kinds, pending.cutoff).then(
                      (moved) => setResult(t('settings.history.done', { count: moved })),
                      (err: unknown) => setResult(String(err)),
                    );
                    setPending(null);
                  }}
                >
                  {t('shelf.trash.ok')}
                </button>
              )}
            </span>
          </div>
        ) : (
          <div className={styles.buttons}>
            <button
              type="button"
              className={styles.button}
              onClick={() => {
                setResult(null);
                countClearable(kinds).then(setPending, (err: unknown) => setResult(String(err)));
              }}
            >
              {t('settings.history.clearButton')}
            </button>
          </div>
        )}
        {result && (
          <p className={styles.notice} role="status">
            {result}
          </p>
        )}
      </Section>
    </>
  );
}
