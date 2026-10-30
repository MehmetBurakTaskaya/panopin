// Prints the updater's latest.json for a release (D-065): the installer's signature and its
// download address on github.com. Plain release URLs need no GitHub API call, so users never meet
// the API's rate limit.
// Usage: node scripts/latest-json.mjs <version> <installer .sig file> <notes file>
import { readFileSync } from 'node:fs';

const [version, signatureFile, notesFile] = process.argv.slice(2);
if (!version || !signatureFile || !notesFile) {
  console.error('usage: node scripts/latest-json.mjs <version> <installer .sig file> <notes file>');
  process.exit(1);
}
const installer = {
  signature: readFileSync(signatureFile, 'utf8').trim(),
  url: `https://github.com/MehmetBurakTaskaya/panopin/releases/download/v${version}/Panopin_${version}_x64-setup.exe`,
};
const latest = {
  version,
  notes: readFileSync(notesFile, 'utf8').trim(),
  // RFC 3339 without fractions.
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
  // The installed app asks for the NSIS entry first; the plain one is the fallback.
  platforms: { 'windows-x86_64-nsis': installer, 'windows-x86_64': installer },
};
process.stdout.write(`${JSON.stringify(latest, null, 2)}\n`);
