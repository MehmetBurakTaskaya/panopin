import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { followSettingsChanges, useSettings } from './state/settings';
import './styles/tokens.css';
import './styles/global.css';
import { applyTheme, followSystemTheme } from './theme';
import { OnboardingApp } from './windows/onboarding/OnboardingApp';
import { SettingsApp } from './windows/settings/SettingsApp';
import { ShelfApp } from './windows/shelf/ShelfApp';

// Every window loads this bundle; the window label picks the view.
const label = getCurrentWebviewWindow().label;
document.documentElement.dataset.window = label;

function View() {
  if (label === 'settings') return <SettingsApp />;
  if (label === 'onboarding') return <OnboardingApp />;
  return <ShelfApp />;
}

async function start(): Promise<void> {
  followSystemTheme();
  useSettings.subscribe((state) => applyTheme(state.settings?.theme ?? 'system'));
  await useSettings.getState().load();
  await followSettingsChanges();
  const root = document.getElementById('root');
  if (!root) return;
  createRoot(root).render(
    <StrictMode>
      <View />
    </StrictMode>,
  );
}

void start();
