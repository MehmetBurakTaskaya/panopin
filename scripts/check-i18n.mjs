// Fails when the locale files disagree or when code uses a key that does not exist.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const load = (lang) => JSON.parse(readFileSync(join(root, 'locales', `${lang}.json`), 'utf8'));
const en = load('en');
const tr = load('tr');
const problems = [];

for (const [name, dict, other] of [['en', en, tr], ['tr', tr, en]]) {
  for (const key of Object.keys(other)) {
    if (!(key in dict)) problems.push(`locales/${name}.json is missing "${key}"`);
  }
  for (const [key, value] of Object.entries(dict)) {
    if (typeof value !== 'string' || value.trim() === '') problems.push(`locales/${name}.json has an empty "${key}"`);
  }
}

// Keys used in code: t('a.b') in TypeScript, i18n::t(lang, "a.b") or t(current_lang(app), "a.b") in Rust.
const pattern = /\bt\(\s*(?:[\w:]+(?:\([^()]*\))?\s*,\s*)?['"]([\w.]+)['"]/g;
function scan(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
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
console.log(`i18n OK: ${Object.keys(en).length} keys in en and tr`);
