// Runs after `npm run build`. Every window loads the same page, so the page itself must link the
// app's one stylesheet, and no other stylesheet may be built that only code loaded later would
// bring in (D-086). Usage: node scripts/check-bundle.mjs [dist folder]
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const dist = process.argv[2] ?? 'dist';
const html = readFileSync(join(dist, 'index.html'), 'utf8');
const attribute = (tag, name) => tag.match(new RegExp(`\\b${name}="/?([^"]+)"`))?.[1];

const linked = [...html.matchAll(/<link\b[^>]*>/g)]
  .map((m) => m[0])
  .filter((tag) => attribute(tag, 'rel') === 'stylesheet')
  .map((tag) => attribute(tag, 'href'));
const built = readdirSync(join(dist, 'assets')).filter((name) => name.endsWith('.css'));
const scripts = [...html.matchAll(/<script\b[^>]*>/g)].map((m) => attribute(m[0], 'src')).filter(Boolean);

const problems = [];
if (linked.length !== 1 || !linked[0]) {
  problems.push(`index.html links ${linked.length} stylesheets, expected 1`);
} else if (!existsSync(join(dist, linked[0]))) {
  problems.push(`index.html links ${linked[0]}, which was not built`);
}
if (built.length !== 1) problems.push(`${dist}/assets holds ${built.length} stylesheets (${built.join(', ')}), expected 1`);
if (scripts.length !== 1) problems.push(`index.html loads ${scripts.length} scripts, expected 1`);

for (const problem of problems) console.error(`check-bundle: ${problem}`);
if (problems.length) process.exit(1);
console.log(`check-bundle: every window gets ${linked[0]} with the page, and its code from ${scripts[0]}`);
