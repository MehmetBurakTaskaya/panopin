import { useEffect, useState } from 'react';
import { t } from '../../i18n';
import { appVersion, openLicenceTexts, openProjectLink } from '../../ipc';
import { Section } from './controls';
import styles from './Settings.module.css';
import thirdParty from './third-party.json';

interface Package {
  name: string;
  version: string;
  license: string;
}

function Packages({ title, list }: { title: string; list: Package[] }) {
  return (
    <details className={styles.packages}>
      <summary>
        {title} ({list.length})
      </summary>
      <table>
        <tbody>
          {list.map((p) => (
            <tr key={`${p.name}@${p.version}`}>
              <td>{p.name}</td>
              <td>{p.version}</td>
              <td>{p.license}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </details>
  );
}

export function AboutTab() {
  const [version, setVersion] = useState('');
  useEffect(() => {
    appVersion().then(setVersion, () => {});
  }, []);
  return (
    <>
      <div className={styles.about}>
        <strong>Panopin</strong>
        <span>{t('settings.about.version', { version })}</span>
        <span>{t('settings.about.madeBy')}</span>
      </div>
      <div className={styles.buttons}>
        <button type="button" className={styles.button} onClick={() => void openProjectLink('website')}>
          mehmetburaktaskaya.com
        </button>
        <button type="button" className={styles.button} onClick={() => void openProjectLink('github')}>
          GitHub
        </button>
        <button type="button" className={styles.button} onClick={() => void openProjectLink('licence')}>
          {t('settings.about.licence')}
        </button>
      </div>
      <Section title={t('settings.about.thirdParty')}>
        <p className={styles.help}>{t('settings.about.thirdPartyHelp')}</p>
        <div className={styles.buttons}>
          <button type="button" className={styles.button} onClick={() => void openLicenceTexts()}>
            {t('settings.about.licenceTexts')}
          </button>
        </div>
        <Packages title="Rust" list={thirdParty.rust} />
        <Packages title="npm" list={thirdParty.npm} />
      </Section>
    </>
  );
}
