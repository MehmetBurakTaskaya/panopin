// Turns a Tauri accelerator ("Super+Alt+V") into the label users expect on their platform.
export type Platform = 'windows' | 'mac';

export function detectPlatform(userAgent: string = navigator.userAgent): Platform {
  return /Mac OS X|Macintosh/.test(userAgent) ? 'mac' : 'windows';
}

const NAMES: Record<Platform, Record<string, string>> = {
  windows: { super: 'Win', alt: 'Alt', control: 'Ctrl', ctrl: 'Ctrl', shift: 'Shift', commandorcontrol: 'Ctrl' },
  mac: { super: '⌘', alt: '⌥', control: '⌃', ctrl: '⌃', shift: '⇧', commandorcontrol: '⌘' },
};

export function formatShortcut(accelerator: string, platform: Platform = detectPlatform()): string {
  const parts = accelerator.split('+').map((part) => {
    const name = NAMES[platform][part.toLowerCase()];
    return name ?? part.toUpperCase();
  });
  return parts.join(platform === 'mac' ? '' : '+');
}
