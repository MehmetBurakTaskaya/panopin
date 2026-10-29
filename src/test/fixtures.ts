import type { Settings } from '../ipc';

/** Settings as the backend returns them by default, for component tests. */
export function testSettings(): Settings {
  return {
    language: 'en',
    theme: 'system',
    shortcuts: {
      shelf: 'Super+Alt+V',
      shelfSearch: 'Super+Alt+C',
      snip: 'Super+Alt+S',
      library: null,
      copyLastImage: null,
      pauseCapture: null,
    },
    onboarded: true,
    sources: {
      screenshots: true,
      clipboardImages: true,
      clipboardText: true,
      links: true,
      screenRecordings: false,
    },
    watchedFolders: [],
    excludedApps: ['KeePass.exe', 'KeePassXC.exe', '1Password.exe', 'Bitwarden.exe'],
    retention: { image: 30, text: 30, link: 30, video: 30, trash: 30 },
    linkPreviews: false,
    pausedUntil: null,
  };
}
