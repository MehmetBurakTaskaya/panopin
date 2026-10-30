// Lists the third-party packages Panopin is built from, with their licences, for Settings → About,
// and writes THIRD_PARTY_NOTICES.txt with every licence text they ship (D-064).
// Run after changing dependencies: `npm run third-party` (needs Node and Cargo).
// `npm run third-party -- --check` only reports whether both files are up to date.
import { execSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const check = process.argv.includes('--check');
const run = (command, cwd = root) => execSync(command, { cwd, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 });

function licenseOf(node) {
  if (typeof node.license === 'string') return node.license;
  if (node.license?.type) return node.license.type;
  if (Array.isArray(node.licenses)) return node.licenses.map((l) => l.type ?? l).join(' OR ');
  return 'UNKNOWN';
}

const npm = JSON.parse(run('npm query .prod'))
  .filter((node) => node.name && node.name !== 'panopin')
  // `location` is relative; npm masks parts of absolute paths that look like ids.
  .map((node) => ({ name: node.name, version: node.version, license: licenseOf(node), dir: path.join(root, node.location) }));

// Every crate the Windows and macOS builds link, as Cargo resolves them.
const TARGETS = ['x86_64-pc-windows-msvc', 'aarch64-apple-darwin', 'x86_64-apple-darwin'];
const tree = TARGETS.map((target) =>
  run(`cargo tree -e normal --target ${target} --prefix none --format "{p}|{l}"`, `${root}/src-tauri`),
).join('\n');
const crateDirs = new Map(
  JSON.parse(run('cargo metadata --format-version 1 --locked', `${root}/src-tauri`)).packages.map((p) => [
    `${p.name}@${p.version}`,
    { dir: path.dirname(p.manifest_path), file: p.license_file, authors: p.authors },
  ]),
);
const rust = tree
  .split(/\r?\n/)
  .map((line) => /^(\S+) v(\S+)[^|]*\|(.*)$/.exec(line.trim()))
  .filter((match) => match && match[1] !== 'panopin')
  // Cargo marks crates it already listed with "(*)".
  .map((match) => ({ name: match[1], version: match[2], license: match[3].replace(/\s*\(\*\)\s*$/, '').trim() || 'UNKNOWN' }))
  .map((p) => ({ ...p, ...crateDirs.get(`${p.name}@${p.version}`) }));

const unique = (list) =>
  [...new Map(list.map((p) => [`${p.name}@${p.version}`, p])).values()].sort(
    (a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version),
  );
const packages = { npm: unique(npm), rust: unique(rust) };

// LICENSE, LICENSE-MIT, LICENCE.md, COPYING, NOTICE, COPYRIGHT…
const LICENCE_FILE = /^(licen[cs]e|copying|notice|copyright)([-._].*)?$/i;
const STANDARD = path.join(root, 'scripts', 'licences');

/** The licence texts a package ships, or the standard texts of its licences when it ships none. */
function textsOf(p) {
  const files = new Set();
  if (p.file) files.add(path.resolve(p.dir, p.file));
  if (p.dir && existsSync(p.dir)) {
    for (const entry of readdirSync(p.dir, { withFileTypes: true })) {
      if (entry.isFile() && LICENCE_FILE.test(entry.name)) files.add(path.join(p.dir, entry.name));
    }
  }
  const shipped = [...files].sort().map((file) => readFileSync(file, 'utf8'));
  if (shipped.length > 0) return { texts: shipped, standard: false };
  const ids = p.license.split(/\s+(?:OR|AND|WITH)\s+|[()/]/).map((id) => id.trim()).filter(Boolean);
  const texts = ids.filter((id) => existsSync(path.join(STANDARD, `${id}.txt`)));
  if (texts.length === 0) throw new Error(`${p.name} ${p.version} ships no licence text and ${p.license} has no standard text`);
  const holders = p.authors?.length ? p.authors.join(', ') : `the ${p.name} authors`;
  return {
    texts: texts.map((id) => readFileSync(path.join(STANDARD, `${id}.txt`), 'utf8').replaceAll('<copyright holders>', holders)),
    standard: true,
  };
}

const normalise = (text) => text.replace(/\r\n?/g, '\n').replace(/[ \t]+$/gm, '').trim();
// Many crates ship the Apache License 2.0 with their own preamble or appendix; its terms are
// printed once at the end, and each copy keeps only what differs.
const APACHE_TERMS = /TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION[\s\S]*?END OF TERMS AND CONDITIONS/;
const APACHE_NOTE = '[The terms of the Apache License, Version 2.0 are printed once, at the end of this file.]';
let apacheTerms = null;
function shorten(text) {
  const terms = APACHE_TERMS.exec(text);
  if (!terms) return text;
  apacheTerms ??= terms[0];
  return text.replace(APACHE_TERMS, APACHE_NOTE);
}

const groups = new Map();
for (const p of [...packages.rust, ...packages.npm]) {
  const { texts, standard } = textsOf(p);
  for (const text of texts.map(normalise).map(shorten)) {
    const group = groups.get(text) ?? { users: [], standard };
    group.users.push(`${p.name} ${p.version}`);
    groups.set(text, group);
  }
}

const version = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const rule = '='.repeat(80);
const sections = [...groups.entries()]
  .map(([text, group]) => ({ text, ...group, users: [...new Set(group.users)] }))
  .sort((a, b) => a.users[0].localeCompare(b.users[0]))
  .map(
    (group) =>
      `${rule}\nUsed by: ${group.users.join(', ')}\n${
        group.standard
          ? `${group.users.length === 1 ? 'This package ships' : 'These packages ship'} no licence file; this is the standard text of its licence.\n`
          : ''
      }${'-'.repeat(80)}\n\n${group.text}\n`,
  );
const notices = `Third-party notices for Panopin ${version}

Panopin is released under the MIT licence (see LICENSE). Its Windows and macOS builds use the
${packages.rust.length} Rust crates and ${packages.npm.length} npm packages listed below, under their own licences. Each
section gives one licence text and the packages that ship it. Where a package offers a choice of
licences, the texts it ships are given.

${sections.join('\n')}${
  apacheTerms ? `\n${rule}\nApache License, Version 2.0: terms and conditions\n${'-'.repeat(80)}\n\n${apacheTerms}\n` : ''
}`;

const list = {
  npm: packages.npm.map(({ name, version, license }) => ({ name, version, license })),
  rust: packages.rust.map(({ name, version, license }) => ({ name, version, license })),
};
const outputs = [
  [path.join(root, 'src', 'windows', 'settings', 'third-party.json'), `${JSON.stringify(list, null, 1)}\n`],
  [path.join(root, 'THIRD_PARTY_NOTICES.txt'), notices],
];

if (check) {
  const stale = outputs.filter(([file, content]) => !existsSync(file) || readFileSync(file, 'utf8') !== content);
  for (const [file] of stale) console.error(`${path.relative(root, file)} is out of date: run npm run third-party`);
  process.exit(stale.length ? 1 : 0);
}
for (const [file, content] of outputs) writeFileSync(file, content);
const standard = [...groups.values()].filter((g) => g.standard).length;
console.log(
  `third-party: ${list.npm.length} npm and ${list.rust.length} Rust packages, ${groups.size} licence texts` +
    (standard ? ` (${standard} standard texts for packages that ship none)` : ''),
);
