// Lists the third-party packages Panopin is built from, with their licences, for Settings → About.
// Run after changing dependencies: `npm run third-party` (needs Node and Cargo).
import { execSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));

function licenseOf(node) {
  if (typeof node.license === 'string') return node.license;
  if (node.license?.type) return node.license.type;
  if (Array.isArray(node.licenses)) return node.licenses.map((l) => l.type ?? l).join(' OR ');
  return 'UNKNOWN';
}

const npm = JSON.parse(execSync('npm query .prod', { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }))
  .filter((node) => node.name && node.name !== 'panopin')
  .map((node) => ({ name: node.name, version: node.version, license: licenseOf(node) }));

// Every crate the Windows build links, as Cargo resolves it.
const tree = execSync('cargo tree -e normal --target x86_64-pc-windows-msvc --prefix none --format "{p}|{l}"', {
  cwd: `${root}/src-tauri`,
  encoding: 'utf8',
  maxBuffer: 64 * 1024 * 1024,
});
const rust = tree
  .split(/\r?\n/)
  .map((line) => /^(\S+) v(\S+)[^|]*\|(.*)$/.exec(line.trim()))
  .filter((match) => match && match[1] !== 'panopin')
  // Cargo marks crates it already listed with "(*)".
  .map((match) => ({ name: match[1], version: match[2], license: match[3].replace(/\s*\(\*\)\s*$/, '').trim() || 'UNKNOWN' }));

const unique = (list) =>
  [...new Map(list.map((p) => [`${p.name}@${p.version}`, p])).values()].sort((a, b) =>
    a.name.localeCompare(b.name) || a.version.localeCompare(b.version),
  );

const out = { npm: unique(npm), rust: unique(rust) };
writeFileSync(`${root}/src/windows/settings/third-party.json`, `${JSON.stringify(out, null, 1)}\n`);
console.log(`third-party list: ${out.npm.length} npm and ${out.rust.length} Rust packages`);
