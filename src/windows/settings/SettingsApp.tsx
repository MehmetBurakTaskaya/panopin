import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { CommandPalette, type PaletteCommand } from '../../components/pickers/CommandPalette';
import { t, type LanguageSetting, type MessageKey } from '../../i18n';
import {
  checkForUpdates,
  setPause,
  systemLook,
  takeSettingsTab,
  type CardSize,
  type Settings,
  type ShelfEdge,
  type SystemLook,
  type Theme,
} from '../../ipc';
import { useSettings } from '../../state/settings';
import { AboutTab } from './AboutTab';
import { Choice, Section, Toggle } from './controls';
import { HistoryTab, SourcesTab } from './DataTabs';
import { PrivacyTab } from './PrivacyTab';
import styles from './Settings.module.css';
import { ShortcutsTab } from './ShortcutsTab';
import { UpdatesTab } from './UpdatesTab';

export const TABS = ['general', 'sources', 'history', 'shortcuts', 'appearance', 'privacy', 'updates', 'about'] as const;
export type TabId = (typeof TABS)[number];

const LANGUAGES: LanguageSetting[] = ['system', 'en', 'tr'];
const CARD_SIZES: CardSize[] = ['s', 'm', 'l'];
const EDGES: ShelfEdge[] = ['top', 'bottom'];
const THEMES: [Theme, MessageKey][] = [
  ['system', 'settings.theme.system'],
  ['light', 'settings.theme.light'],
  ['dark', 'settings.theme.dark'],
];

function GeneralTab({ settings }: { settings: Settings }) {
  const update = useSettings((s) => s.update);
  return (
    <>
      <Section title={t('settings.general.app')}>
        <Toggle label={t('settings.launchAtLogin')} checked={settings.launchAtLogin} onChange={(on) => void update({ launchAtLogin: on })} />
        <Choice
          label={t('settings.language')}
          value={settings.language}
          options={LANGUAGES.map((language) => [language, t(`settings.language.${language}`)])}
          onChange={(value) => void update({ language: value as LanguageSetting })}
        />
      </Section>
      <Section title={t('settings.shelf')}>
        <Choice
          label={t('settings.shelfEdge')}
          value={settings.shelfEdge}
          options={EDGES.map((edge) => [edge, t(`settings.shelfEdge.${edge}`)])}
          onChange={(value) => void update({ shelfEdge: value as ShelfEdge })}
        />
        <Toggle
          label={t('settings.edgeOpen')}
          hint={t('settings.edgeOpenHelp')}
          checked={settings.edgeOpen}
          onChange={(on) => void update({ edgeOpen: on })}
        />
        <Toggle
          label={t('settings.searchWithoutAccents')}
          hint={t('settings.searchWithoutAccentsHelp')}
          checked={settings.searchWithoutAccents}
          onChange={(on) => void update({ searchWithoutAccents: on })}
        />
      </Section>
      <Section title={t('settings.sounds')}>
        <Toggle label={t('settings.soundsOn')} checked={settings.sounds.enabled} onChange={(on) => void update({ sounds: { enabled: on } })} />
        <div className={styles.row}>
          <label className={styles.label} htmlFor="volume">
            <span>{t('settings.soundsVolume')}</span>
          </label>
          <input
            id="volume"
            type="range"
            min={0}
            max={100}
            step={5}
            value={settings.sounds.volume}
            disabled={!settings.sounds.enabled}
            onChange={(event) => void update({ sounds: { volume: Number(event.target.value) } })}
          />
        </div>
      </Section>
    </>
  );
}

function AppearanceTab({ settings, look }: { settings: Settings; look: SystemLook | null }) {
  const update = useSettings((s) => s.update);
  const appearance = settings.appearance;
  return (
    <>
      <Section title={t('settings.appearance.colours')}>
        <Choice
          label={t('settings.theme')}
          value={settings.theme}
          options={THEMES.map(([theme, label]) => [theme, t(label)])}
          onChange={(value) => void update({ theme: value as Theme })}
        />
        <Toggle
          label={t('settings.systemAccent')}
          hint={look?.accentLight ? <span className={styles.swatch} style={{ background: look.accentLight }} /> : undefined}
          checked={appearance.systemAccent}
          onChange={(on) => void update({ appearance: { systemAccent: on } })}
        />
      </Section>
      <Section title={t('settings.shelf')}>
        <Choice
          label={t('settings.cardSize')}
          value={settings.cardSize}
          options={CARD_SIZES.map((size) => [size, t(`settings.cardSize.${size}`)])}
          onChange={(value) => void update({ cardSize: value as CardSize })}
        />
      </Section>
      <Section title={t('settings.appearance.windows')}>
        <Toggle
          label={t('settings.material')}
          hint={t(look?.mica === false ? 'settings.materialUnavailable' : 'settings.materialHelp')}
          checked={appearance.material && look?.mica !== false}
          disabled={look?.mica === false}
          onChange={(on) => void update({ appearance: { material: on } })}
        />
        <Toggle
          label={t('settings.animations')}
          hint={t('settings.animationsHelp')}
          checked={appearance.animations}
          onChange={(on) => void update({ appearance: { animations: on } })}
        />
      </Section>
    </>
  );
}

/** Settings: tabs on the left, every change applies at once (D-060, D-061). */
export function SettingsApp() {
  useSettings((s) => s.lang);
  const settings = useSettings((s) => s.settings);
  const error = useSettings((s) => s.error);
  const [tab, setTab] = useState<TabId>('general');
  const [look, setLook] = useState<SystemLook | null>(null);
  const [palette, setPalette] = useState<PaletteCommand[] | null>(null);
  const update = useSettings((s) => s.update);
  const tabs = useRef<Partial<Record<TabId, HTMLButtonElement | null>>>({});
  useEffect(() => {
    systemLook().then(setLook, () => {});
  }, []);
  // The tray opens Settings on a tab: a new window asks once, an open one hears an event.
  useEffect(() => {
    const show = (id: string | null) => {
      if (id && (TABS as readonly string[]).includes(id)) setTab(id as TabId);
    };
    takeSettingsTab().then(show, () => {});
    const off = listen<string>('settings:tab', (event) => show(event.payload));
    return () => void off.then((stop) => stop(), () => {});
  }, []);

  // Ctrl+K: go to a tab or flip a setting from the keyboard (D-063).
  const commands = (current: Settings): PaletteCommand[] => {
    const minutes = (n: number) => () => void setPause(Date.now() + n * 60_000);
    const list: [string, string, () => void][] = [
      ...TABS.map((id): [string, string, () => void] => [t('palette.goTab', { tab: t(`settings.tab.${id}`) }), '', () => setTab(id)]),
      [t(current.linkPreviews ? 'palette.linkPreviewsOff' : 'palette.linkPreviewsOn'), '', () => void update({ linkPreviews: !current.linkPreviews })],
      [t(current.ocr ? 'palette.ocrOff' : 'palette.ocrOn'), '', () => void update({ ocr: !current.ocr })],
      [`${t('tray.pause')}: ${t('tray.pause.5m')}`, '', minutes(5)],
      [`${t('tray.pause')}: ${t('tray.pause.1h')}`, '', minutes(60)],
      [t('tray.resume'), '', () => void setPause(null)],
      [
        t('tray.checkUpdates'),
        '',
        () => {
          setTab('updates');
          void checkForUpdates();
        },
      ],
      ...THEMES.map(([theme, label]): [string, string, () => void] => [
        `${t('settings.theme')}: ${t(label)}`,
        '',
        () => void update({ theme }),
      ]),
    ];
    return list.map(([label, shortcut, run], i) => ({ id: `s${i}`, label, shortcut: shortcut || undefined, run }));
  };
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.code === 'KeyK' && settings) {
        e.preventDefault();
        setPalette(commands(settings));
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
  if (!settings) return null;

  const onTabKey = (e: KeyboardEvent<HTMLButtonElement>) => {
    const at = TABS.indexOf(tab);
    const next =
      e.key === 'ArrowDown' ? TABS[(at + 1) % TABS.length] : e.key === 'ArrowUp' ? TABS[(at + TABS.length - 1) % TABS.length] : e.key === 'Home' ? TABS[0] : e.key === 'End' ? TABS[TABS.length - 1] : null;
    if (!next) return;
    e.preventDefault();
    setTab(next);
    tabs.current[next]?.focus();
  };

  return (
    <div className={styles.app}>
      <nav className={styles.nav} role="tablist" aria-orientation="vertical" aria-label={t('settings.title')}>
        {TABS.map((id) => (
          <button
            key={id}
            ref={(el) => {
              tabs.current[id] = el;
            }}
            type="button"
            role="tab"
            id={`tab-${id}`}
            aria-selected={tab === id}
            aria-controls="settings-panel"
            tabIndex={tab === id ? 0 : -1}
            className={styles.tab}
            onClick={() => setTab(id)}
            onKeyDown={onTabKey}
          >
            {t(`settings.tab.${id}`)}
          </button>
        ))}
      </nav>
      <main className={styles.page} role="tabpanel" id="settings-panel" aria-labelledby={`tab-${tab}`}>
        <h1>{t(`settings.tab.${tab}`)}</h1>
        {tab === 'general' && <GeneralTab settings={settings} />}
        {tab === 'sources' && <SourcesTab settings={settings} />}
        {tab === 'history' && <HistoryTab settings={settings} />}
        {tab === 'shortcuts' && <ShortcutsTab settings={settings} />}
        {tab === 'appearance' && <AppearanceTab settings={settings} look={look} />}
        {tab === 'privacy' && <PrivacyTab settings={settings} />}
        {tab === 'updates' && <UpdatesTab settings={settings} />}
        {tab === 'about' && <AboutTab />}
        {error && (
          <p className={styles.error} role="alert">
            {t('settings.saveFailed', { error })}
          </p>
        )}
      </main>
      {palette && (
        <CommandPalette
          commands={palette}
          onClose={() => {
            setPalette(null);
            tabs.current[tab]?.focus();
          }}
        />
      )}
    </div>
  );
}
