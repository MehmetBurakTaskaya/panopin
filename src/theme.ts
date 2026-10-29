import type { Theme } from './ipc';

const darkQuery = () => matchMedia('(prefers-color-scheme: dark)');
let current: Theme = 'system';

/** Sets `data-scheme` on <html>; `system` follows Windows' app mode and tracks changes. */
export function applyTheme(theme: Theme): void {
  current = theme;
  const scheme = theme === 'system' ? (darkQuery().matches ? 'dark' : 'light') : theme;
  document.documentElement.dataset.scheme = scheme;
}

export function followSystemTheme(): void {
  darkQuery().addEventListener('change', () => applyTheme(current));
}
