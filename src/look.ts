// The look every window follows (D-060): Mica behind the Library and Settings, the Windows accent
// colour, and animations (off, or as Windows sets them).
import { listen } from '@tauri-apps/api/event';
import { systemLook, type Settings, type SystemLook } from './ipc';

let look: SystemLook | null = null;
let settings: Settings | null = null;

function apply(): void {
  const root = document.documentElement;
  if (!settings) return;
  const appearance = settings.appearance;
  root.dataset.material = look?.mica && appearance.material ? 'mica' : '';
  root.dataset.motion = appearance.animations ? '' : 'off';
  if (appearance.systemAccent && look?.accentLight && look.accentDark) {
    root.dataset.accent = 'system';
    root.style.setProperty('--system-accent-light', look.accentLight);
    root.style.setProperty('--system-accent-dark', look.accentDark);
  } else {
    root.dataset.accent = '';
  }
}

export function applyLook(next: Settings | null): void {
  settings = next;
  apply();
}

/** Reads what Windows offers once, then follows accent colour changes. */
export async function followLook(): Promise<void> {
  try {
    look = await systemLook();
  } catch {
    look = null;
  }
  apply();
  await listen<SystemLook>('look:changed', ({ payload }) => {
    look = payload;
    apply();
  });
}

/** Animations are off when the setting says so or Windows asks for reduced motion. */
export function reducedMotion(): boolean {
  return document.documentElement.dataset.motion === 'off' || matchMedia('(prefers-reduced-motion: reduce)').matches;
}
