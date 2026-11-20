// Holds a run of the W3C VC-JOSE-COSE suite to what `localConfig.cjs` declares.
//
//   node check-run.mjs <report.json>
//
// Run from the root of the suite checkout, after `mocha --reporter json` has written
// the report. It reads `localConfig.cjs` and `tests/test-mapping.js` from there, so
// the expected rows come from the suite's own table and the declared features, never
// from a number written here.
//
// Why it exists. The suite's own exit status is not enough: mocha passes a run in which
// every row was skipped, and the runner skips a row for any feature the manifest does
// not declare `true`, a typo or a missing key included. So this checks, from the
// machine-readable report:
//
//   - every one of the six features is declared, as a boolean;
//   - exactly one implementation ran, and it is the one declared;
//   - every row of an enabled feature ran and passed, which in the suite means the
//     implementation's output was the verdict the suite expects;
//   - every row of a disabled feature was skipped, and nothing else was;
//   - the report holds no other test, no failure and no duplicate;
//   - at least one row ran;
//   - an issuance that succeeded left a compact JWS in `data`, which the suite does
//     not look at: three base64url segments, and a first one that decodes to a JSON
//     object with a string `alg`. The signature is not verified.
//
// The expected count is the number of rows of the enabled features. It is not a
// constant, so it follows the feature list when a later change turns more on.

import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const require = createRequire(import.meta.url);

// The three suites of `tests/test-runner.js`, by the title it gives each `describe`.
const SUITES = [
  ['JOSE Tests', 'JOSETestMapping'],
  ['SD-JWT Tests', 'SDJWTTestMapping'],
  ['COSE Tests', 'COSETestMapping'],
];

const problems = [];
const problem = (message) => problems.push(message);

// The shape of an issuance's `data`: a compact JWS, checked as far as its shape goes.
const SEGMENT = /^[A-Za-z0-9_-]+$/;

// The reason `token` is not a compact JWS, or null when its shape is one.
function notCompactJws(token) {
  if (typeof token !== 'string') return 'data is not a string';
  const segments = token.split('.');
  if (segments.length !== 3 || !segments.every((segment) => SEGMENT.test(segment))) {
    return 'it is not three non-empty base64url segments';
  }
  const header = decodeHeader(segments[0]);
  if (header === null || typeof header !== 'object' || Array.isArray(header) || typeof header.alg !== 'string') {
    return 'its first segment does not decode to a JSON object with a string alg';
  }
  return null;
}

// The JSON that a base64url segment holds, or null when it is not canonical base64url
// or not JSON. A re-encode that differs means the segment was not base64url to begin with.
function decodeHeader(segment) {
  const bytes = Buffer.from(segment, 'base64url');
  if (bytes.toString('base64url') !== segment) return null;
  try {
    return JSON.parse(bytes.toString('utf8'));
  } catch {
    return null;
  }
}

const reportPath = process.argv[2];
if (!reportPath) {
  console.error('usage: node check-run.mjs <report.json>');
  process.exit(2);
}

const mapping = await import(pathToFileURL(resolve('tests/test-mapping.js')));
const manifest = require(resolve('localConfig.cjs'));
const report = JSON.parse(readFileSync(reportPath, 'utf8'));

const declared = manifest.implementations ?? [];
if (declared.length !== 1) {
  console.error(`localConfig.cjs must declare exactly one implementation, it declares ${declared.length}`);
  process.exit(1);
}
const name = declared[0].name;
const features = declared[0]['jose-cose']?.features ?? {};

for (const feature of Object.values(mapping.TestFeature)) {
  if (typeof features[feature] !== 'boolean') {
    problem(`feature ${feature} is not declared true or false: the suite would skip its rows without saying so`);
  }
}
for (const feature of Object.keys(features)) {
  if (!Object.values(mapping.TestFeature).includes(feature)) {
    problem(`feature ${feature} is declared and is not one the suite has`);
  }
}

// What must happen, row by row, from the suite's table and the declared features.
const rows = new Map();
for (const [suite, exportName] of SUITES) {
  for (const [testName, config] of Object.entries(mapping[exportName])) {
    rows.set(`${suite} ${name} ${testName}`, {testName, config, enabled: features[config.feature] === true});
  }
}
const enabled = [...rows].filter(([, row]) => row.enabled).map(([title]) => title);
const disabled = [...rows].filter(([, row]) => !row.enabled).map(([title]) => title);

const titles = (list) => list.map((test) => test.fullTitle);
const passed = titles(report.passes ?? []);
const skipped = titles(report.pending ?? []);
const everything = titles(report.tests ?? []);

if ((report.failures ?? []).length > 0) {
  for (const test of report.failures) {
    problem(`failed: ${test.fullTitle}: ${test.err?.message ?? 'no message'}`);
  }
}
if (new Set(everything).size !== everything.length) {
  problem('the report lists a test more than once');
}
for (const title of everything) {
  if (!rows.has(title)) {
    problem(`ran something that is not a declared row of ${name}: ${title}`);
  }
}
for (const title of enabled) {
  if (!passed.includes(title)) {
    problem(`enabled row did not run and pass: ${title}`);
  }
}
for (const title of disabled) {
  if (!skipped.includes(title)) {
    problem(`disabled row was not skipped: ${title}`);
  }
}
if (enabled.length === 0) {
  problem('no feature is enabled, so nothing was run');
}
if (passed.length !== enabled.length) {
  problem(`${passed.length} rows passed and ${enabled.length} were expected to`);
}
if (skipped.length !== disabled.length) {
  problem(`${skipped.length} rows were skipped and ${disabled.length} were expected to be`);
}

// The verdict of each row that ran, as the implementation wrote it.
console.log(`${name}: ${enabled.length} rows run, ${disabled.length} skipped, of ${rows.size}`);
for (const title of enabled) {
  const {testName, config} = rows.get(title);
  let output = {};
  try {
    output = JSON.parse(readFileSync(resolve(`tests/output/${config.number}-${name}.json`), 'utf8'));
  } catch (error) {
    problem(`row ${config.number} left no readable output: ${error.message}`);
  }
  if (output === null || typeof output !== 'object' || Array.isArray(output)) {
    problem(`row ${config.number} left output that is not a JSON object`);
    output = {};
  }
  if (config.fn === 'issue' && output.result === 'success') {
    const why = notCompactJws(output.data);
    if (why) problem(`row ${config.number} issued, and its data is not a compact JWS: ${why}`);
  }
  const reason = output.reason ? `  (${output.reason})` : '';
  console.log(`  ${testName}: ${output.result}, expected ${config.expected_result}${reason}`);
}
for (const title of disabled) {
  console.log(`  ${rows.get(title).testName}: skipped, feature ${rows.get(title).config.feature} is off`);
}

if (problems.length > 0) {
  console.error('');
  for (const message of problems) {
    console.error(`ERROR: ${message}`);
  }
  process.exit(1);
}
console.log('every row of every enabled feature ran and got the verdict the suite expects');
