// Tests for check-run.mjs. Each case builds a suite root in a temporary directory: the
// real `tests/test-mapping.js` of a suite checkout, a `localConfig.cjs`, a mocha-style
// `report.json` and the `tests/output` files of the rows that ran. It runs check-run.mjs
// there as a child process and checks the exit status and the message.
//
//   SUITE_ROOT=target/vc-jose-cose-test-suite node --test .github/oracle/vc-jose-cose/check-run.test.mjs

import {after, test} from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

const checker = join(dirname(fileURLToPath(import.meta.url)), 'check-run.mjs');

if (!process.env.SUITE_ROOT) {
  throw new Error('SUITE_ROOT must name a checkout of the W3C VC-JOSE-COSE test suite');
}
const mappingSource = join(resolve(process.env.SUITE_ROOT), 'tests', 'test-mapping.js');
const mapping = await import(pathToFileURL(mappingSource));

const IMPL = 'dpp-vc';

// The three suites of `tests/test-runner.js`, by the title it gives each `describe`.
const SUITES = [
  ['JOSE Tests', mapping.JOSETestMapping],
  ['SD-JWT Tests', mapping.SDJWTTestMapping],
  ['COSE Tests', mapping.COSETestMapping],
];

// The features as `localConfig.cjs` declares them today.
const FEATURES = {
  credential_jose: true,
  presentation_jose: true,
  credential_sdjwt: true,
  presentation_sdjwt: true,
  credential_cose: true,
  presentation_cose: true,
};
const ALL_OFF = Object.fromEntries(Object.keys(FEATURES).map((feature) => [feature, false]));

const rowsOf = (features) =>
  SUITES.flatMap(([suite, table]) =>
    Object.entries(table).map(([testName, config]) => ({
      testName,
      config,
      title: `${suite} ${IMPL} ${testName}`,
      enabled: features[config.feature] === true,
    })),
  );

const ROWS = rowsOf(FEATURES);
const ENABLED = ROWS.filter((row) => row.enabled);
const ISSUED = ENABLED.find((row) => row.config.fn === 'issue' && row.config.expected_result === 'success');

const outputPath = (number) => `tests/output/${number}-${IMPL}.json`;

const compactJws = (header, payload = {}) => {
  const encode = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');
  return `${encode(header)}.${encode(payload)}.c2lnbmF0dXJl`;
};

// One Disclosure, `[salt, name, value]`, as RFC 9901 section 4.2.1 encodes it.
const DISCLOSURE = Buffer.from(JSON.stringify(['c2FsdA', 'name', 'value'])).toString('base64url');
const sdJwt = (jws = compactJws({alg: 'ES256'})) => `${jws}~${DISCLOSURE}~`;

// A tagged COSE_Sign1 as the harness writes one, as the standard-alphabet base64 without padding that
// the suite's own COSE inputs use: tag 18, an array of four, a protected header map holding `alg`
// (-7), an empty unprotected header, a payload and a 64-byte signature.
const cborBytes = (bytes) => Buffer.concat([Buffer.from([0x40 + bytes.length]), bytes]);
const coseBytes = ({
  tag = [0xd2],
  array = [0x84],
  protectedItem = cborBytes(Buffer.from([0xa1, 0x01, 0x26])),
  unprotected = [0xa0],
  payloadItem = cborBytes(Buffer.from('{}')),
  tail = [],
} = {}) =>
  Buffer.concat([
    Buffer.from(tag),
    Buffer.from(array),
    protectedItem,
    Buffer.from(unprotected),
    payloadItem,
    Buffer.from([0x58, 0x40]),
    Buffer.alloc(64, 7),
    Buffer.from(tail),
  ]);
const unpadded = (bytes) => bytes.toString('base64').replace(/=+$/, '');
const coseText = (options) => unpadded(coseBytes(options));

// What a row's output file holds when the harness gave the verdict the suite expects.
function outputFor(config) {
  if (config.expected_result !== 'success') {
    return {result: config.expected_result, reason: 'refused'};
  }
  if (config.fn !== 'issue') return {result: 'success'};
  if (config.feature.endsWith('_cose')) return {result: 'success', data: coseText()};
  return {result: 'success', data: config.feature.endsWith('_sdjwt') ? sdJwt() : compactJws({alg: 'ES256'})};
}

function entry(row) {
  return {
    title: row.testName,
    fullTitle: row.title,
    file: 'tests/test-runner.js',
    duration: 1,
    currentRetry: 0,
    speed: 'fast',
    err: {},
  };
}

// A run in which every enabled row ran and passed, and the disabled rows were skipped.
// `ran` is the feature set the run was made with.
function standardRun(ran) {
  const rows = rowsOf(ran);
  const enabled = rows.filter((row) => row.enabled);
  return {
    tests: rows.map(entry),
    passes: enabled.map(entry),
    pending: rows.filter((row) => !row.enabled).map(entry),
    failures: [],
    outputs: new Map(enabled.map((row) => [outputPath(row.config.number), outputFor(row.config)])),
  };
}

const roots = [];
after(() => {
  for (const root of roots) rmSync(root, {recursive: true, force: true});
});

// A suite root for check-run.mjs. `ran` is the feature set the run was made with, and
// `declared` the one `localConfig.cjs` states. They differ only where a case says so.
function suiteRoot({ran = FEATURES, declared = ran, edit = () => {}} = {}) {
  const run = standardRun(ran);
  edit(run);
  const root = mkdtempSync(join(tmpdir(), 'check-run-'));
  roots.push(root);
  mkdirSync(join(root, 'tests', 'output'), {recursive: true});
  copyFileSync(mappingSource, join(root, 'tests', 'test-mapping.js'));
  writeFileSync(join(root, 'package.json'), '{"type": "module"}\n');
  writeFileSync(join(root, 'localConfig.cjs'), manifest(declared));
  writeFileSync(
    join(root, 'report.json'),
    JSON.stringify({
      stats: {
        tests: run.tests.length,
        passes: run.passes.length,
        pending: run.pending.length,
        failures: run.failures.length,
      },
      tests: run.tests,
      passes: run.passes,
      pending: run.pending,
      failures: run.failures,
    }),
  );
  for (const [file, body] of run.outputs) {
    writeFileSync(join(root, file), JSON.stringify(body));
  }
  return root;
}

function manifest(features) {
  const config = {implementations: [{name: IMPL, implementation: IMPL, 'jose-cose': {features}}]};
  return `module.exports = ${JSON.stringify(config, null, 2)};\n`;
}

function check(root) {
  return spawnSync(process.execPath, [checker, 'report.json'], {cwd: root, encoding: 'utf8'});
}

// The check failed (exit 1) and said so, with this message.
function assertRefused(result, message) {
  assert.equal(result.status, 1, `exit ${result.status}\n${result.stdout}${result.stderr}`);
  assert.ok(result.stderr.includes(message), `no "${message}" in:\n${result.stderr}`);
}

test('a run that got the expected verdict on every enabled row exits 0', () => {
  const result = check(suiteRoot());
  assert.equal(result.status, 0, result.stderr);
  const skipped = ROWS.length - ENABLED.length;
  assert.ok(
    result.stdout.includes(`${IMPL}: ${ENABLED.length} rows run, ${skipped} skipped, of ${ROWS.length}`),
    result.stdout,
  );
  assert.ok(result.stdout.includes('every row of every enabled feature ran'), result.stdout);
});

test('every row of all six features is expected to run, and none is skipped', () => {
  assert.equal(ENABLED.length, ROWS.length);
  assert.equal(ROWS.length, 35);
  const result = check(suiteRoot());
  assert.equal(result.status, 0, result.stderr);
  assert.ok(result.stdout.includes(`${IMPL}: 35 rows run, 0 skipped, of 35`), result.stdout);
});

test('a feature the manifest does not declare is refused by name', () => {
  const declared = {...FEATURES};
  delete declared.credential_cose;
  assertRefused(check(suiteRoot({declared})), 'feature credential_cose is not declared true or false');
});

test('a feature switched on whose rows did not run is refused', () => {
  const ran = {...FEATURES, credential_cose: false};
  const row = ROWS.find((candidate) => candidate.config.feature === 'credential_cose');
  assertRefused(check(suiteRoot({ran, declared: FEATURES})), `enabled row did not run and pass: ${row.title}`);
});

test('a feature switched off whose rows ran is not skipped, and is refused', () => {
  const declared = {...FEATURES, presentation_cose: false};
  const row = ROWS.find((candidate) => candidate.config.feature === 'presentation_cose');
  assertRefused(check(suiteRoot({ran: FEATURES, declared})), `disabled row was not skipped: ${row.title}`);
});

test('a manifest with every feature off is refused, not passed with nothing run', () => {
  assertRefused(check(suiteRoot({ran: ALL_OFF})), 'no feature is enabled, so nothing was run');
});

test('a failure in the report is refused with its message', () => {
  const root = suiteRoot({
    edit: (run) => {
      const [failed] = run.passes.splice(0, 1);
      run.failures.push({...failed, err: {message: 'the signature did not verify'}});
    },
  });
  assertRefused(check(root), `failed: ${ENABLED[0].title}: the signature did not verify`);
});

test('a test from another implementation is refused', () => {
  const title = `JOSE Tests other-impl ${ENABLED[0].testName}`;
  const root = suiteRoot({
    edit: (run) => {
      const stranger = {...run.passes[0], fullTitle: title};
      run.tests.push(stranger);
      run.passes.push(stranger);
    },
  });
  assertRefused(check(root), `ran something that is not a declared row of ${IMPL}: ${title}`);
});

test('a title listed twice in the report is refused', () => {
  const root = suiteRoot({edit: (run) => run.tests.push(run.tests[0])});
  assertRefused(check(root), 'the report lists a test more than once');
});

test('an enabled row with no output file is refused', () => {
  const root = suiteRoot({edit: (run) => run.outputs.delete(outputPath(ISSUED.config.number))});
  assertRefused(check(root), `row ${ISSUED.config.number} left no readable output`);
});

test('an enabled row whose output is JSON null is refused by name', () => {
  const root = suiteRoot({edit: (run) => run.outputs.set(outputPath(ISSUED.config.number), null)});
  assertRefused(check(root), `row ${ISSUED.config.number} left output that is not a JSON object`);
});

const NOT_THREE = 'it is not three non-empty base64url segments';
const NOT_A_HEADER = 'its first segment does not decode to a JSON object with a string alg';
const issuances = [
  ['data is not a string', 42, 'data is not a string'],
  ['data is ".."', '..', NOT_THREE],
  ['data has an empty segment', `${compactJws({alg: 'ES256'}).split('.')[0]}..c2ln`, NOT_THREE],
  ['data has a character outside base64url', 'a+b.c.d', NOT_THREE],
  ['the header is not JSON', `${Buffer.from('not json').toString('base64url')}.e30.c2ln`, NOT_A_HEADER],
  ['the header is not an object', compactJws([]), NOT_A_HEADER],
  ['the header names no alg', compactJws({typ: 'vc+jwt'}), NOT_A_HEADER],
  [
    'the header has a stray trailing character',
    `${compactJws({alg: 'ES256'}).split('.')[0]}A.e30.c2ln`,
    NOT_A_HEADER,
  ],
];
for (const [name, data, why] of issuances) {
  test(`an issuance in which ${name} is refused`, () => {
    const root = suiteRoot({
      edit: (run) => run.outputs.set(outputPath(ISSUED.config.number), {result: 'success', data}),
    });
    assertRefused(check(root), `row ${ISSUED.config.number} issued, and its data is not a compact JWS: ${why}`);
  });
}

// An SD-JWT feature's issuance must leave an SD-JWT: the compact JWS above, then its
// Disclosures, then the closing `~`.
const SD_ISSUED = ENABLED.find(
  (row) => row.config.fn === 'issue' && row.config.expected_result === 'success' && row.config.feature.endsWith('_sdjwt'),
);
const NO_TILDE = 'it does not end with the `~` of an SD-JWT without Key Binding';
const BAD_DISCLOSURE = 'a Disclosure is not a non-empty base64url segment';
const sdIssuances = [
  ['is a bare compact JWS', compactJws({alg: 'ES256'}), NO_TILDE],
  ['has no closing ~', `${compactJws({alg: 'ES256'})}~${DISCLOSURE}`, NO_TILDE],
  ['has an Issuer-signed JWT of ".."', '..~', 'its Issuer-signed JWT: it is not three non-empty base64url segments'],
  ['has a Disclosure outside base64url', `${compactJws({alg: 'ES256'})}~a+b~`, BAD_DISCLOSURE],
  ['has an empty Disclosure', `${compactJws({alg: 'ES256'})}~~${DISCLOSURE}~`, BAD_DISCLOSURE],
  ['is not a string', 42, 'data is not a string'],
];
for (const [name, data, why] of sdIssuances) {
  test(`an SD-JWT issuance that ${name} is refused`, () => {
    const root = suiteRoot({
      edit: (run) => run.outputs.set(outputPath(SD_ISSUED.config.number), {result: 'success', data}),
    });
    assertRefused(check(root), `row ${SD_ISSUED.config.number} issued, and its data is not an SD-JWT: ${why}`);
  });
}

test('an SD-JWT issuance with no Disclosures is accepted', () => {
  const root = suiteRoot({
    edit: (run) => run.outputs.set(outputPath(SD_ISSUED.config.number), {result: 'success', data: `${compactJws({alg: 'ES256'})}~`}),
  });
  assert.equal(check(root).status, 0);
});

// A COSE feature's issuance must leave the base64 text of a COSE_Sign1, which the checker reads by
// its CBOR head bytes: an optional tag 18, an array of four, and the four items' types.
const COSE_ISSUED = ENABLED.find(
  (row) => row.config.fn === 'issue' && row.config.expected_result === 'success' && row.config.feature.endsWith('_cose'),
);
const NOT_BASE64 = 'it is not base64 of the standard alphabet with no padding';
const NOT_CANONICAL = 'it is not canonical base64';
const NOT_AN_ARRAY = 'it does not decode to a CBOR array of four items, tagged 18 or not';
const NOT_A_MAP = 'its protected header is not empty and not one encoded CBOR map';
const NOT_WELL_FORMED = 'the message ends inside an item or holds one that is not well formed';
const coseIssuances = [
  ['is not a string', 42, 'data is not a string'],
  ['is padded', coseBytes().toString('base64').padEnd(Math.ceil(coseText().length / 4) * 4, '='), NOT_BASE64],
  ['is base64url', `${coseText().slice(0, 10)}-${coseText().slice(11)}`, NOT_BASE64],
  ['has a line break', `${coseText().slice(0, 8)}\n${coseText().slice(8)}`, NOT_BASE64],
  ['is empty', '', NOT_BASE64],
  ['has a length of one modulo four', `${coseText()}AAA`, NOT_CANONICAL],
  ['is a compact JWS', compactJws({alg: 'ES256'}), NOT_BASE64],
  ['is JSON text', unpadded(Buffer.from('{"a":1}')), NOT_AN_ARRAY],
  ['is the hex of a COSE_Sign1, which is also base64', coseBytes().toString('hex'), NOT_AN_ARRAY],
  ['carries a tag other than 18', coseText({tag: [0xd1]}), 'it carries a CBOR tag other than 18'],
  ['is an array of three', coseText({array: [0x83]}), NOT_AN_ARRAY],
  ['is an array of five', coseText({array: [0x85]}), NOT_AN_ARRAY],
  ['is an indefinite-length array', coseText({array: [0x9f]}), NOT_AN_ARRAY],
  ['has an unprotected header that is not a map', coseText({unprotected: [0x80]}), 'its unprotected header is not a CBOR map'],
  ['has a detached payload', coseText({payloadItem: Buffer.from([0xf6])}), 'its payload is not a byte string'],
  ['has bytes after the array', coseText({tail: [0x00]}), 'bytes follow the COSE_Sign1 array'],
  ['ends inside the signature', unpadded(coseBytes().subarray(0, -3)), 'the message ends inside an item'],
  // RFC 9052 section 3: the protected header is empty or the encoding of one header map.
  ['has a protected header that holds an integer', coseText({protectedItem: cborBytes(Buffer.from([0x01]))}), NOT_A_MAP],
  ['has bytes after its protected header map', coseText({protectedItem: cborBytes(Buffer.from([0xa1, 0x01, 0x26, 0x00]))}), NOT_A_MAP],
  // RFC 8949 section 3.3: a simple value below 32 in the two-byte form is not well formed.
  ['writes false in two bytes', coseText({unprotected: [0xa1, 0x18, 0x63, 0xf8, 0x14]}), NOT_WELL_FORMED],
];
for (const [name, data, why] of coseIssuances) {
  test(`a COSE issuance that ${name} is refused`, () => {
    const root = suiteRoot({
      edit: (run) => run.outputs.set(outputPath(COSE_ISSUED.config.number), {result: 'success', data}),
    });
    assertRefused(check(root), `row ${COSE_ISSUED.config.number} issued, and its data is not a COSE_Sign1: ${why}`);
  });
}

test('an untagged COSE_Sign1 is accepted, since RFC 9052 section 4.2 allows it', () => {
  const root = suiteRoot({
    edit: (run) => run.outputs.set(outputPath(COSE_ISSUED.config.number), {result: 'success', data: coseText({tag: []})}),
  });
  assert.equal(check(root).status, 0);
});

test('an empty protected header, and a simple value written in two bytes from 32 up, are accepted', () => {
  for (const data of [
    coseText({protectedItem: Buffer.from([0x40])}),
    coseText({unprotected: [0xa1, 0x18, 0x63, 0xf8, 0x20]}),
  ]) {
    const root = suiteRoot({
      edit: (run) => run.outputs.set(outputPath(COSE_ISSUED.config.number), {result: 'success', data}),
    });
    assert.equal(check(root).status, 0, data);
  }
});

// The report says the row passed, and its output file must say the same: a verdict that
// differs means the two files are not of one run, and the token in the output is not the one
// the suite judged.
test('an output whose verdict is not the one the row expects is refused, whatever the report says', () => {
  const refusedRow = ENABLED.find((row) => row.config.expected_result === 'failure');
  for (const [row, result] of [
    [COSE_ISSUED, 'failure'],
    [refusedRow, 'success'],
  ]) {
    const root = suiteRoot({
      edit: (run) => run.outputs.set(outputPath(row.config.number), {result}),
    });
    assertRefused(
      check(root),
      `row ${row.config.number} left the verdict ${result}, and the row expects ${row.config.expected_result}`,
    );
  }
});

test('a COSE issuance is not judged as a compact JWS, nor a JOSE one as COSE', () => {
  const jose = ENABLED.find((row) => row.config.fn === 'issue' && row.config.feature === 'credential_jose');
  const root = suiteRoot({
    edit: (run) => run.outputs.set(outputPath(jose.config.number), {result: 'success', data: coseText()}),
  });
  assertRefused(check(root), `row ${jose.config.number} issued, and its data is not a compact JWS`);
});
