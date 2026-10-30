import { useEffect, useState } from 'react';
import { formatLocale, t } from '../../i18n';
import { ocrStatus, openProjectLink, setPause, type OcrStatus, type Settings } from '../../ipc';
import { useSettings } from '../../state/settings';
import { Section, Toggle } from './controls';
import styles from './Settings.module.css';

const FOREVER = Number.MAX_SAFE_INTEGER;

/** Excluded apps, pausing, link previews and OCR, with what each one means for privacy. */
export function PrivacyTab({ settings }: { settings: Settings }) {
  const update = useSettings((s) => s.update);
  const [app, setApp] = useState('');
  const [ocr, setOcr] = useState<OcrStatus | null>(null);
  useEffect(() => {
    ocrStatus().then(setOcr, () => {});
  }, []);
  const apps = settings.excludedApps;
  const addApp = () => {
    const name = app.trim();
    if (!name) return;
    const exe = /\.exe$/i.test(name) ? name : `${name}.exe`;
    if (!apps.some((known) => known.toLowerCase() === exe.toLowerCase())) void update({ excludedApps: [...apps, exe] });
    setApp('');
  };
  const pausedUntil = settings.pausedUntil;
  const paused = pausedUntil !== null && pausedUntil > Date.now();
  const time = new Intl.DateTimeFormat(formatLocale(), { timeStyle: 'short' });
  const minutes = (n: number) => () => void setPause(Date.now() + n * 60_000);

  return (
    <>
      <Section title={t('settings.privacy.excluded')}>
        <p className={styles.help}>{t('settings.privacy.excludedHelp')}</p>
        {apps.map((name) => (
          <div key={name} className={styles.row}>
            <span className={styles.path}>{name}</span>
            <button type="button" className={styles.button} onClick={() => void update({ excludedApps: apps.filter((a) => a !== name) })}>
              {t('settings.sources.remove')}
            </button>
          </div>
        ))}
        <div className={styles.row}>
          <input
            className={styles.input}
            value={app}
            placeholder="App.exe"
            aria-label={t('settings.privacy.addApp')}
            onChange={(e) => setApp(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && addApp()}
          />
          <button type="button" className={styles.button} onClick={addApp}>
            {t('settings.privacy.addApp')}
          </button>
        </div>
      </Section>

      <Section title={t('settings.privacy.pause')}>
        <div className={styles.row}>
          <span className={styles.label}>
            <span>
              {!paused
                ? t('settings.privacy.capturing')
                : pausedUntil >= FOREVER
                  ? t('settings.privacy.pausedForever')
                  : t('settings.privacy.pausedUntil', { time: time.format(pausedUntil) })}
            </span>
          </span>
          <span className={styles.buttons}>
            {paused ? (
              <button type="button" className={styles.button} onClick={() => void setPause(null)}>
                {t('tray.resume')}
              </button>
            ) : (
              <>
                <button type="button" className={styles.button} onClick={minutes(5)}>
                  {t('tray.pause.5m')}
                </button>
                <button type="button" className={styles.button} onClick={minutes(60)}>
                  {t('tray.pause.1h')}
                </button>
                <button type="button" className={styles.button} onClick={() => void setPause(FOREVER)}>
                  {t('tray.pause.untilResumed')}
                </button>
              </>
            )}
          </span>
        </div>
      </Section>

      <Section title={t('settings.privacy.network')}>
        <p className={styles.help}>{t('onboarding.updates')}</p>
        <Toggle
          label={t('onboarding.linkPreviews')}
          hint={t('settings.privacy.linkPreviewsHelp')}
          checked={settings.linkPreviews}
          onChange={(on) => void update({ linkPreviews: on })}
        />
        <Toggle
          label={t('settings.privacy.ocr')}
          hint={
            ocr && !ocr.supported
              ? t('settings.privacy.ocrUnsupported')
              : ocr?.reading
                ? t('settings.privacy.ocrReading', {
                    language: ocr.languages.find((l) => l.tag === ocr.reading || ocr.reading?.startsWith(l.tag))?.name ?? ocr.reading,
                  })
                : t('settings.privacy.ocrHelp')
          }
          checked={settings.ocr}
          disabled={ocr?.supported === false}
          onChange={(on) => void update({ ocr: on })}
        />
        {ocr?.supported && !ocr.languages.some((l) => /^(tr|en)/i.test(l.tag)) && (
          <div className={styles.row}>
            <span className={styles.label}>
              <small>{t('settings.privacy.ocrAddLanguage')}</small>
            </span>
            <button type="button" className={styles.button} onClick={() => void openProjectLink('languages')}>
              {t('settings.privacy.ocrOpenLanguages')}
            </button>
          </div>
        )}
      </Section>

      <p className={styles.help}>{t('onboarding.notVault')}</p>
    </>
  );
}
