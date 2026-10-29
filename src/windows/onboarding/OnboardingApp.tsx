import { useState } from 'react';
import { t } from '../../i18n';
import type { Sources } from '../../ipc';
import { useSettings } from '../../state/settings';
import styles from './Onboarding.module.css';

const SOURCES = ['screenshots', 'clipboardImages', 'clipboardText', 'links'] as const;

/** First run: choose what to collect. Nothing is captured until "Start" is pressed. */
export function OnboardingApp() {
  useSettings((s) => s.lang);
  const settings = useSettings((s) => s.settings);
  const error = useSettings((s) => s.error);
  const update = useSettings((s) => s.update);
  const [sources, setSources] = useState<Sources | null>(settings?.sources ?? null);
  const [linkPreviews, setLinkPreviews] = useState(settings?.linkPreviews ?? false);
  if (!sources) return null;

  return (
    <main className={styles.page}>
      <h1>{t('onboarding.title')}</h1>
      <p className={styles.lead}>{t('onboarding.intro')}</p>

      <section className={styles.card} role="group" aria-labelledby="onboarding-sources">
        <h2 id="onboarding-sources">{t('onboarding.sources')}</h2>
        {SOURCES.map((key) => (
          <label key={key} className={styles.option}>
            <input
              type="checkbox"
              checked={sources[key]}
              onChange={() => setSources({ ...sources, [key]: !sources[key] })}
            />
            <span>{t(`onboarding.source.${key}`)}</span>
          </label>
        ))}
        <p className={styles.note}>{t('onboarding.textNote')}</p>
      </section>

      <section className={styles.card}>
        <h2>{t('onboarding.privacyTitle')}</h2>
        <p>{t('onboarding.privacy')}</p>
        <p className={styles.strong}>{t('onboarding.notVault')}</p>
      </section>

      <label className={`${styles.card} ${styles.option}`}>
        <input type="checkbox" checked={linkPreviews} onChange={() => setLinkPreviews(!linkPreviews)} />
        <span>
          <strong>{t('onboarding.linkPreviews')}</strong>
          <small>{t('onboarding.linkPreviewsHelp')}</small>
        </span>
      </label>

      {error && (
        <p className={styles.error} role="alert">
          {t('settings.saveFailed', { error })}
        </p>
      )}

      <button
        type="button"
        className={styles.start}
        onClick={() => void update({ onboarded: true, sources, linkPreviews })}
      >
        {t('onboarding.start')}
      </button>
    </main>
  );
}
