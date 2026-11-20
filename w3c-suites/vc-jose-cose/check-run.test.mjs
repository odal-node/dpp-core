// Tests for check-run.mjs. Each case builds a suite root in a temporary directory: the
// real `tests/test-mapping.js` of a suite checkout, a `localConfig.cjs`, a mocha-style
// `report.json` and the `tests/output` files of the rows that ran. It runs check-run.mjs
// there as a child process and checks the exit status and the message.
//
//   SUITE_ROOT=target/vc-jose-cose-test-suite node --test w3c-suites/vc-jose-cose/check-run.test.mjs

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
  credential_sdjwt: false,
  presentation_sdjwt: false,
  credential_cose: false,
  presentation_cose: false,
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

// What a row's output file holds when the harness gave the verdict the suite expects.
function outputFor(config) {
  if (config.expected_result !== 'success') {
    return {result: config.expected_result, reason: 'refused'};
  }
  return config.fn === 'issue' ? {result: 'success', data: compactJws({alg: 'ES256'})} : {result: 'success'};
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

test('a feature the manifest does not declare is refused by name', () => {
  const declared = {...FEATURES};
  delete declared.credential_cose;
  assertRefused(check(suiteRoot({declared})), 'feature credential_cose is not declared true or false');
});

test('a feature switched on whose rows did not run is refused', () => {
  const declared = {...FEATURES, credential_sdjwt: true};
  const row = ROWS.find((candidate) => candidate.config.feature === 'credential_sdjwt');
  assertRefused(check(suiteRoot({declared})), `enabled row did not run and pass: ${row.title}`);
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
