import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { StrictMode, type ComponentType } from 'react';
import { createRoot } from 'react-dom/client';
import { followSettingsChanges, useSettings } from './state/settings';
import './styles/tokens.css';
import './styles/global.css';
import { applyLook, followLook } from './look';
import { applyTheme, followSystemTheme } from './theme';

// Every window starts here; the window label picks the view, and only that view's code is loaded.
const label = getCurrentWebviewWindow().label;
document.documentElement.dataset.window = label;

function loadView(): Promise<ComponentType> {
  if (label === 'settings') return import('./windows/settings/SettingsApp').then((m) => m.SettingsApp);
  if (label === 'onboarding') return import('./windows/onboarding/OnboardingApp').then((m) => m.OnboardingApp);
  if (label === 'library') return import('./windows/library/LibraryApp').then((m) => m.LibraryApp);
  return import('./windows/shelf/ShelfApp').then((m) => m.ShelfApp);
}

async function start(): Promise<void> {
  followSystemTheme();
  useSettings.subscribe((state) => {
    applyTheme(state.settings?.theme ?? 'system');
    applyLook(state.settings);
  });
  // The view's code, the settings and the system's look arrive together, not one after another.
  const [View] = await Promise.all([loadView(), useSettings.getState().load(), followLook(), followSettingsChanges()]);
  const root = document.getElementById('root');
  if (!root) return;
  createRoot(root).render(
    <StrictMode>
      <View />
    </StrictMode>,
  );
}

void start();
