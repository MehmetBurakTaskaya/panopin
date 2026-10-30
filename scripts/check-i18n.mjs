// Fails when a locale file disagrees with locales/en.json (keys, {placeholders}, empty texts, its
// "@language" and "@status"), when the app's language lists and the files differ, or when code
// uses a key that does not exist. Lists the translations that still wait for review.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const dir = join(root, 'locales');
const codes = readdirSync(dir)
  .filter((file) => file.endsWith('.json'))
  .map((file) => file.slice(0, -'.json'.length))
  .sort();
const load = (code) => JSON.parse(readFileSync(join(dir, `${code}.json`), 'utf8'));
const en = load('en');
const problems = [];
const waiting = [];
const placeholders = (text) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');

for (const code of codes) {
  const file = `locales/${code}.json`;
  let dict;
  try {
    dict = load(code);
  } catch (err) {
    problems.push(`${file} is not valid JSON: ${err.message}`);
    continue;
  }
  const status = dict['@status'];
  if (!dict['@language']) problems.push(`${file} has no "@language"`);
  if (code === 'en' ? status !== 'source' : !['needs review', 'reviewed'].includes(status)) {
    problems.push(`${file} has "@status" ${JSON.stringify(status)}; use "needs review" or "reviewed"`);
  }
  if (status === 'needs review') waiting.push(code);
  for (const key of Object.keys(en)) {
    if (!(key in dict)) problems.push(`${file} is missing "${key}"`);
  }
  for (const [key, value] of Object.entries(dict)) {
    if (!(key in en)) {
      problems.push(`${file} has "${key}", which locales/en.json does not`);
    } else if (typeof value !== 'string' || value.trim() === '') {
      problems.push(`${file} has an empty "${key}"`);
    } else if (!key.startsWith('@') && placeholders(value) !== placeholders(en[key])) {
      problems.push(`${file} changes the {placeholders} of "${key}"`);
    }
  }
}

// The language list in src/i18n and the Rust side load every file, and nothing else.
const listed = [...readFileSync(join(root, 'src', 'i18n', 'index.ts'), 'utf8').matchAll(/code: '([\w-]+)'/g)]
  .map((m) => m[1])
  .sort();
if (listed.join() !== codes.join()) {
  problems.push(`LANGUAGES in src/i18n/index.ts (${listed.join(', ')}) differs from locales/ (${codes.join(', ')})`);
}
const rust = readFileSync(join(root, 'src-tauri', 'src', 'i18n.rs'), 'utf8');
for (const code of codes) {
  if (!rust.includes(`locales/${code}.json`)) problems.push(`src-tauri/src/i18n.rs does not load locales/${code}.json`);
}

// Keys used in code: t('a.b') in TypeScript, i18n::t(lang, "a.b") or t(current_lang(app), "a.b") in Rust.
const pattern = /\bt\(\s*(?:[\w:]+(?:\([^()]*\))?\s*,\s*)?['"]([\w.]+)['"]/g;
function scan(folder) {
  for (const entry of readdirSync(folder)) {
    const path = join(folder, entry);
    if (statSync(path).isDirectory()) {
      scan(path);
    } else if (/\.(tsx?|rs)$/.test(entry) && !/\.test\.tsx?$/.test(entry)) {
      // Rust unit tests live at the end of each file and use made-up keys on purpose.
      const source = readFileSync(path, 'utf8').split(/#\[cfg\(test\)\]\r?\nmod tests/)[0];
      for (const [, key] of source.matchAll(pattern)) {
        if (!(key in en)) problems.push(`${relative(root, path)} uses unknown key "${key}"`);
      }
    }
  }
}
scan(join(root, 'src'));
scan(join(root, 'src-tauri', 'src'));

if (problems.length > 0) {
  console.error(problems.join('\n'));
  process.exit(1);
}
const texts = Object.keys(en).filter((key) => !key.startsWith('@')).length;
console.log(`i18n OK: ${texts} texts in ${codes.length} languages; waiting for review: ${waiting.join(', ') || 'none'}`);
