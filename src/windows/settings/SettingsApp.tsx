import { t, type LanguageSetting } from '../../i18n';
import type { CardSize, ShelfEdge } from '../../ipc';
import { formatShortcut } from '../../shortcuts';
import { useSettings } from '../../state/settings';
import styles from './Settings.module.css';

const LANGUAGES: LanguageSetting[] = ['system', 'en', 'tr'];
const CARD_SIZES: CardSize[] = ['s', 'm', 'l'];
const EDGES: ShelfEdge[] = ['top', 'bottom'];

function ShortcutLabel({ accelerator }: { accelerator: string | null }) {
  return accelerator ? <kbd>{formatShortcut(accelerator)}</kbd> : <span>{t('settings.shortcut.off')}</span>;
}

/** General settings; every change applies at once. Shortcuts are read-only until the recorder arrives. */
export function SettingsApp() {
  useSettings((s) => s.lang);
  const settings = useSettings((s) => s.settings);
  const error = useSettings((s) => s.error);
  const update = useSettings((s) => s.update);
  if (!settings) return null;

  return (
    <main className={styles.page}>
      <h1>{t('settings.general')}</h1>
      <label className={styles.row}>
        <span>{t('settings.language')}</span>
        <select
          value={settings.language}
          onChange={(event) => void update({ language: event.target.value as LanguageSetting })}
        >
          {LANGUAGES.map((language) => (
            <option key={language} value={language}>
              {t(`settings.language.${language}`)}
            </option>
          ))}
        </select>
      </label>

      <h2>{t('settings.shelf')}</h2>
      <div className={styles.group}>
        <label className={styles.row}>
          <span>{t('settings.cardSize')}</span>
          <select
            value={settings.cardSize}
            onChange={(event) => void update({ cardSize: event.target.value as CardSize })}
          >
            {CARD_SIZES.map((size) => (
              <option key={size} value={size}>
                {t(`settings.cardSize.${size}`)}
              </option>
            ))}
          </select>
        </label>
        <label className={styles.row}>
          <span>{t('settings.shelfEdge')}</span>
          <select
            value={settings.shelfEdge}
            onChange={(event) => void update({ shelfEdge: event.target.value as ShelfEdge })}
          >
            {EDGES.map((edge) => (
              <option key={edge} value={edge}>
                {t(`settings.shelfEdge.${edge}`)}
              </option>
            ))}
          </select>
        </label>
        <label className={styles.row}>
          <span>{t('settings.sounds')}</span>
          <input
            type="checkbox"
            checked={settings.sounds.enabled}
            onChange={(event) => void update({ sounds: { enabled: event.target.checked } })}
          />
        </label>
        <label className={styles.row}>
          <span>{t('settings.soundsVolume')}</span>
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={settings.sounds.volume}
            disabled={!settings.sounds.enabled}
            onChange={(event) => void update({ sounds: { volume: Number(event.target.value) } })}
          />
        </label>
      </div>

      <h2>{t('settings.shortcuts')}</h2>
      <dl className={styles.shortcuts}>
        <dt>{t('settings.shortcut.shelf')}</dt>
        <dd>
          <ShortcutLabel accelerator={settings.shortcuts.shelf} />
        </dd>
        <dt>{t('settings.shortcut.shelfSearch')}</dt>
        <dd>
          <ShortcutLabel accelerator={settings.shortcuts.shelfSearch} />
        </dd>
      </dl>

      {error && (
        <p className={styles.error} role="alert">
          {t('settings.saveFailed', { error })}
        </p>
      )}
    </main>
  );
}
