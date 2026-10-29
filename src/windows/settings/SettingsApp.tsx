import { t, type LanguageSetting } from '../../i18n';
import { formatShortcut } from '../../shortcuts';
import { useSettings } from '../../state/settings';
import styles from './Settings.module.css';

const LANGUAGES: LanguageSetting[] = ['system', 'en', 'tr'];

function ShortcutLabel({ accelerator }: { accelerator: string | null }) {
  return accelerator ? <kbd>{formatShortcut(accelerator)}</kbd> : <span>{t('settings.shortcut.off')}</span>;
}

/** Phase 1 settings: language (applies instantly) and a read-only view of the shortcuts. */
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
