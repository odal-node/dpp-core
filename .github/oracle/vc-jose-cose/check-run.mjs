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
//     object with a string `alg`. For the SD-JWT features it is an SD-JWT: that
//     compact JWS, then its Disclosures, each base64url, then a closing `~`. For
//     the COSE features it is the base64 of a COSE_Sign1, in the form the suite's own
//     COSE inputs have (the standard alphabet, no padding), read by its CBOR head
//     bytes: an optional tag 18, an array of four items, and a byte string, a map, a
//     byte string and a byte string in them, with nothing after. No signature is
//     verified.
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

// The head of the CBOR item at `at`: its major type, its argument, and where the item's content
// starts. `null` for a head that is cut short or is an indefinite length or a reserved form,
// which the harness never writes, and for a simple value below 32 in the two-byte form, which
// RFC 8949 section 3.3 calls not well formed. The argument is a Number, which is exact for
// every length a message of this size can have.
function cborHead(bytes, at) {
  if (at >= bytes.length) return null;
  const major = bytes[at] >> 5;
  const info = bytes[at] & 0x1f;
  if (info < 24) return {major, argument: info, next: at + 1};
  const width = {24: 1, 25: 2, 26: 4, 27: 8}[info];
  if (width === undefined || at + 1 + width > bytes.length) return null;
  const argument = Number(BigInt('0x' + bytes.subarray(at + 1, at + 1 + width).toString('hex')));
  if (major === 7 && info === 24 && argument < 32) return null;
  return {major, argument, next: at + 1 + width};
}

// The position after the CBOR item at `at`, or null when it is cut short.
function cborSkip(bytes, at) {
  const head = cborHead(bytes, at);
  if (head === null) return null;
  const {major, argument} = head;
  let next = head.next;
  if (major === 2 || major === 3) return next + argument <= bytes.length ? next + argument : null;
  if (major === 4 || major === 5) {
    for (let i = 0; i < argument * (major === 5 ? 2 : 1); i += 1) {
      next = cborSkip(bytes, next);
      if (next === null) return null;
    }
    return next;
  }
  if (major === 6) return cborSkip(bytes, next);
  return next;
}

// The base64 the suite writes its COSE inputs in: the standard alphabet with no padding.
const COSE_TEXT = /^[A-Za-z0-9+/]+$/;

// The reason `token` is not the text of a COSE_Sign1 (RFC 9052 section 4.2), or null when it is
// one as far as the head bytes say: tagged 18 or not, an array of four items that are a byte
// string (the protected header) that is empty or holds one encoded map and nothing after it
// (RFC 9052 section 3), a map (the unprotected header), a byte string (the payload) and a byte
// string (the signature), and nothing after them.
function notCose(token) {
  if (typeof token !== 'string') return 'data is not a string';
  if (!COSE_TEXT.test(token)) return 'it is not base64 of the standard alphabet with no padding';
  const bytes = Buffer.from(token, 'base64');
  if (bytes.toString('base64').replace(/=+$/, '') !== token) return 'it is not canonical base64';

  let at = 0;
  let head = cborHead(bytes, at);
  if (head !== null && head.major === 6) {
    if (head.argument !== 18) return 'it carries a CBOR tag other than 18';
    at = head.next;
    head = cborHead(bytes, at);
  }
  if (head === null || head.major !== 4 || head.argument !== 4) {
    return 'it does not decode to a CBOR array of four items, tagged 18 or not';
  }
  at = head.next;
  const items = [
    [2, 'its protected header is not a byte string'],
    [5, 'its unprotected header is not a CBOR map'],
    [2, 'its payload is not a byte string'],
    [2, 'its signature is not a byte string'],
  ];
  const cutShort = 'the message ends inside an item or holds one that is not well formed';
  for (const [index, [major, why]] of items.entries()) {
    head = cborHead(bytes, at);
    if (head === null) return cutShort;
    if (head.major !== major) return why;
    const end = cborSkip(bytes, at);
    if (end === null) return cutShort;
    if (index === 0 && end > head.next) {
      const header = bytes.subarray(head.next, end);
      const map = cborHead(header, 0);
      if (map === null || map.major !== 5 || cborSkip(header, 0) !== header.length) {
        return 'its protected header is not empty and not one encoded CBOR map';
      }
    }
    at = end;
  }
  if (at !== bytes.length) return 'bytes follow the COSE_Sign1 array';
  return null;
}

// `<Issuer-signed JWT>~<Disclosure>~...~`: a compact JWS, then the Disclosures, then a
// closing `~` because no Key Binding JWT follows (RFC 9901 section 4).
function notSdJwt(token) {
  if (typeof token !== 'string') return 'data is not a string';
  if (!token.endsWith('~')) return 'it does not end with the `~` of an SD-JWT without Key Binding';
  const [jwt, ...rest] = token.split('~');
  const why = notCompactJws(jwt);
  if (why) return `its Issuer-signed JWT: ${why}`;
  if (!rest.slice(0, -1).every((disclosure) => SEGMENT.test(disclosure))) {
    return 'a Disclosure is not a non-empty base64url segment';
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
  // The report says the row passed. Its output file must say the same, or the two are not of
  // one run, and a token in the output is not the one the suite judged.
  if (output.result !== config.expected_result) {
    problem(`row ${config.number} left the verdict ${output.result}, and the row expects ${config.expected_result}`);
  }
  if (config.fn === 'issue' && output.result === 'success') {
    if (config.feature.endsWith('_cose')) {
      const why = notCose(output.data);
      if (why) problem(`row ${config.number} issued, and its data is not a COSE_Sign1: ${why}`);
    } else if (config.feature.endsWith('_sdjwt')) {
      const why = notSdJwt(output.data);
      if (why) problem(`row ${config.number} issued, and its data is not an SD-JWT: ${why}`);
    } else {
      const why = notCompactJws(output.data);
      if (why) problem(`row ${config.number} issued, and its data is not a compact JWS: ${why}`);
    }
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
