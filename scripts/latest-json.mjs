// Prints the updater's latest.json for a release (D-065): each platform's signature and its
// download address on github.com. Plain release URLs need no GitHub API call, so users never meet
// the API's rate limit.
// Usage: node scripts/latest-json.mjs <version> <notes file> <folder with the release files>
// The folder holds Panopin-Setup-x64.exe and, once macOS ships, the macOS update archive, each
// next to its .sig file.
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

const [version, notesFile, folder] = process.argv.slice(2);
if (!version || !notesFile || !folder) {
  console.error('usage: node scripts/latest-json.mjs <version> <notes file> <folder>');
  process.exit(1);
}
const base = `https://github.com/MehmetBurakTaskaya/panopin/releases/download/v${version}`;
const files = readdirSync(folder);
function entry(pattern) {
  const name = files.find((file) => pattern.test(file) && existsSync(path.join(folder, `${file}.sig`)));
  if (!name) return null;
  return { signature: readFileSync(path.join(folder, `${name}.sig`), 'utf8').trim(), url: `${base}/${encodeURIComponent(name)}` };
}

const platforms = {};
const windows = entry(/^Panopin-Setup-x64\.exe$/);
if (!windows) {
  console.error('the Windows installer or its signature is missing');
  process.exit(1);
}
// The installed app asks for the NSIS entry first; the plain one is the fallback.
platforms['windows-x86_64-nsis'] = windows;
platforms['windows-x86_64'] = windows;
// One universal archive serves both Mac architectures.
const mac = entry(/\.app\.tar\.gz$/);
if (mac) {
  platforms['darwin-aarch64'] = mac;
  platforms['darwin-x86_64'] = mac;
}

const latest = {
  version,
  notes: readFileSync(notesFile, 'utf8').trim(),
  // RFC 3339 without fractions.
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
  platforms,
};
process.stdout.write(`${JSON.stringify(latest, null, 2)}\n`);
