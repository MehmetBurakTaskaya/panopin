// Draws every icon from src-tauri/icons/app-icon.svg: the app icon in all sizes (`tauri icon`),
// the tray pictures for each state at each small-icon size (D-067) and the website's icons.
// Run after changing the SVG: `npm run icons`.
import { execSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const icons = path.join(root, 'src-tauri', 'icons');
const source = path.join(icons, 'app-icon.svg');
const work = mkdtempSync(path.join(tmpdir(), 'panopin-icons-'));
const tauriIcon = (args) => execSync(`npx tauri icon ${args}`, { cwd: root, stdio: ['ignore', 'ignore', 'inherit'] });

try {
  // The app icon; only the files the Windows and macOS bundles use are kept.
  tauriIcon(`"${source}" -o "${path.join(work, 'app')}"`);
  for (const file of ['32x32.png', '64x64.png', '128x128.png', '128x128@2x.png', 'icon.png', 'icon.ico', 'icon.icns']) {
    copyFileSync(path.join(work, 'app', file), path.join(icons, file));
  }

  // Tray states: greyed with a pause badge while capture is paused, a blue dot when an update is
  // on offer. Each is drawn at 16, 20, 24 and 32 px (100 % to 200 % scaling).
  const svg = readFileSync(source, 'utf8');
  const inner = svg.replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '');
  const wrap = (body) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">${body}</svg>`;
  const greyed = `<defs><filter id="grey"><feColorMatrix type="saturate" values="0"/></filter></defs><g filter="url(#grey)" opacity=".9">${inner}</g>`;
  const pause =
    '<circle cx="190" cy="190" r="60" fill="#2f2f2f" stroke="#fff" stroke-width="12"/>' +
    '<rect x="166" y="160" width="17" height="60" rx="4" fill="#fff"/><rect x="197" y="160" width="17" height="60" rx="4" fill="#fff"/>';
  const dot = '<circle cx="206" cy="50" r="42" fill="#0f7ae5" stroke="#fff" stroke-width="12"/>';
  const states = {
    normal: inner,
    update: inner + dot,
    paused: greyed + pause,
    'paused-update': greyed + pause + dot,
  };
  mkdirSync(path.join(icons, 'tray'), { recursive: true });
  for (const [name, body] of Object.entries(states)) {
    const file = path.join(work, `${name}.svg`);
    writeFileSync(file, wrap(body));
    tauriIcon(`"${file}" --png 16,20,24,32 -o "${path.join(work, name)}"`);
    for (const size of [16, 20, 24, 32]) {
      copyFileSync(path.join(work, name, `${size}x${size}.png`), path.join(icons, 'tray', `${name}-${size}.png`));
    }
  }

  // The macOS menu bar wants template pictures: black on transparent, which macOS tints for light
  // and dark menu bars (D-073). A note with its pin; paused is faint with a pause badge, an update
  // adds a dot. 44 px is 22 pt at 2x.
  const note =
    '<g transform="rotate(-6 32 38)"><rect x="12" y="20" width="40" height="36" rx="5" fill="none" stroke="#000" stroke-width="5"/>' +
    '<path d="M20 35h24M20 44h14" stroke="#000" stroke-width="4" stroke-linecap="round"/></g>' +
    '<circle cx="32" cy="17" r="9" fill="#000"/>';
  const macPause = '<rect x="41" y="40" width="6" height="20" rx="1.5" fill="#000"/><rect x="51" y="40" width="6" height="20" rx="1.5" fill="#000"/>';
  const macDot = '<circle cx="54" cy="12" r="8" fill="#000"/>';
  const template = (body) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">${body}</svg>`;
  const macStates = {
    normal: note,
    update: note + macDot,
    paused: `<g opacity=".45">${note}</g>${macPause}`,
    'paused-update': `<g opacity=".45">${note}</g>${macPause}${macDot}`,
  };
  for (const [name, body] of Object.entries(macStates)) {
    const file = path.join(work, `mac-${name}.svg`);
    writeFileSync(file, template(body));
    tauriIcon(`"${file}" --png 44 -o "${path.join(work, `mac-${name}`)}"`);
    copyFileSync(path.join(work, `mac-${name}`, '44x44.png'), path.join(icons, 'tray', `mac-${name}.png`));
  }

  // The website and the READMEs use the same picture.
  copyFileSync(source, path.join(root, 'site', 'assets', 'panopin.svg'));
  copyFileSync(path.join(icons, '32x32.png'), path.join(root, 'site', 'assets', 'favicon-32.png'));
  console.log('icons: app icon, 4 tray states × 4 sizes, 4 macOS menu bar templates, website icons');
} finally {
  rmSync(work, { recursive: true, force: true });
}
