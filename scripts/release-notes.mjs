// Prints the CHANGELOG.md section of one version: the notes Settings shows for an update.
// With --release it adds the download notes for the GitHub release page.
// Usage: node scripts/release-notes.mjs 1.0.0 [--release]
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const [version, flag] = process.argv.slice(2);
const changelog = readFileSync(fileURLToPath(new URL('../CHANGELOG.md', import.meta.url)), 'utf8').replace(/\r\n?/g, '\n');
const heading = new RegExp(`^## \\[${version.replaceAll('.', '\\.')}\\][^\\n]*\\n`, 'm');
const start = heading.exec(changelog);
if (!start) {
  console.error(`CHANGELOG.md has no section for ${version}`);
  process.exit(1);
}
const rest = changelog.slice(start.index + start[0].length);
const end = rest.search(/^## \[|^\[[^\]]+\]: /m);
const notes = (end === -1 ? rest : rest.slice(0, end)).trim();

const downloads = `### Downloads

- **Panopin-Setup-x64.exe**: the installer for Windows 10 22H2 and Windows 11 (x64). It installs for
  your user only and needs no administrator rights.
- **Panopin-Portable-x64.zip**: unzip it anywhere and run \`Panopin.exe\`.

Panopin is not code-signed yet. If SmartScreen says "Windows protected your PC", choose
**More info → Run anyway**. \`SHA256SUMS.txt\` lists the SHA-256 of every file.`;

process.stdout.write(`${flag === '--release' ? `${notes}\n\n${downloads}` : notes}\n`);
