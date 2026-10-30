// Prints the CHANGELOG.md section of one version: the notes Settings shows for an update.
// With --release it adds the download notes for the GitHub release page; --mac adds the Mac files
// to them, for releases that carry them.
// Usage: node scripts/release-notes.mjs 1.0.0 [--release] [--mac]
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const [version, ...flags] = process.argv.slice(2);
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

const mac = `
- **Panopin-macOS.dmg**: for macOS 13 or later, on Apple silicon and Intel. Open it and drag Panopin
  into Applications. The first time, macOS cannot check the app: open **System Settings → Privacy &
  Security** and choose **Open Anyway**. \`Panopin-macOS.app.tar.gz\` is what updates install.`;

const downloads = `### Downloads

- **Panopin-Setup-x64.exe**: the installer for Windows 10 22H2 and Windows 11 (x64). Open it and
  Panopin installs for your user, with no administrator rights and no pages to click through, then
  starts.
- **Panopin-Portable-x64.zip**: unzip it anywhere and run \`Panopin.exe\`.${flags.includes('--mac') ? mac : ''}

Panopin is not code-signed yet. If SmartScreen says "Windows protected your PC", choose
**More info → Run anyway**. \`SHA256SUMS.txt\` lists the SHA-256 of every file.`;

process.stdout.write(`${flags.includes('--release') ? `${notes}\n\n${downloads}` : notes}\n`);
