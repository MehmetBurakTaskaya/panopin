// Fails when a production npm dependency (anything shipped in the app bundle) has a licence
// outside the project's allow-list. Dev-only tooling never ships and is not checked here.
import { execSync } from 'node:child_process';

const ALLOWED = new Set([
  'MIT', 'Apache-2.0', 'BSD-2-Clause', 'BSD-3-Clause', 'ISC', 'Zlib', 'Unicode-3.0', 'MPL-2.0', '0BSD',
]);

// An SPDX expression passes when every AND-part has at least one allowed OR-alternative.
function allowed(expression) {
  const clean = expression.replace(/[()]/g, ' ');
  return clean.split(/\s+AND\s+/).every((part) => part.split(/\s+OR\s+/).some((id) => ALLOWED.has(id.trim())));
}

function licenseOf(node) {
  if (typeof node.license === 'string') return node.license;
  if (node.license?.type) return node.license.type;
  if (Array.isArray(node.licenses)) return node.licenses.map((l) => l.type ?? l).join(' OR ');
  return 'UNKNOWN';
}

// `npm query .prod` lists every installed production package with its package.json fields.
const nodes = JSON.parse(
  execSync('npm query .prod', { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }),
);

const packages = new Map(nodes.map((node) => [`${node.name}@${node.version}`, licenseOf(node)]));
const bad = [...packages].filter(([, license]) => !allowed(license));
for (const [id, license] of packages) console.log(`${allowed(license) ? 'ok ' : 'BAD'} ${id} ${license}`);
if (bad.length > 0) {
  console.error(`\n${bad.length} production package(s) with a disallowed licence.`);
  process.exit(1);
}
console.log(`\nnpm licences OK: ${packages.size} production packages`);
