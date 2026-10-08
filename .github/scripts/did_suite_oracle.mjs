/**
 * Run the W3C DID test suite over the DID documents `build_did_document` produces.
 *
 * The documents come from `dpp-vc`'s `did_suite_input` test, in the input format
 * the suite defines for an implementation. This script puts that file where the
 * suite looks for one, runs the suite's `did-core-properties` and
 * `did-production` suites, and holds the result to three things:
 *
 *   it passes    no failed test, no skipped one, no suite that crashed
 *   it was run   every produced DID went through the assertions the claim rests
 *                on, in both suites, so an input file that quietly came out empty
 *                or misnamed cannot turn into a green job
 *   it can fail  the same documents with two rules broken on purpose make the
 *                suite go red, so a suite that has stopped checking anything
 *                (a matcher that always passes, a renamed property) is caught
 *
 * The suite is somebody else's reading of DID Core, which is the point: our own
 * tests were written by whoever wrote the builder.
 *
 * **What passing does not say.** The suite checks the documents. It checks no
 * resolution and no DID method, and most of its rules about `service`,
 * `controller` and the other verification relationships are vacuous for
 * documents that carry none of them, which is why those are not in the list of
 * assertions required below. The register records the scope of the claim.
 *
 * Configuration is the suite's documented route, not a patch: its README tells an
 * implementer to add their input file to `default.js` of each suite. Passing the
 * input as a jest global instead would work, and would fail the suite's own
 * `toBeInfraMap`, which is an `instanceof Object` check and so rejects every
 * object created outside the test sandbox.
 *
 * Usage: node did_suite_oracle.mjs <implementation.json> <did-test-suite checkout>
 *
 * Exits non-zero listing every problem, not just the first.
 */

import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const [inputPath, suiteRoot] = process.argv.slice(2);
if (!inputPath || !suiteRoot) {
  console.error("usage: did_suite_oracle.mjs <implementation.json> <did-test-suite checkout>");
  process.exit(2);
}

const server = path.resolve(suiteRoot, "packages/did-core-test-server");
const suites = path.join(server, "suites");
const IMPLEMENTATION = "did-web-odal.json";

// The two suites the claim rests on, and the one-line configuration each reads.
// Each lists this implementation alone, so a run is about our documents and does
// not wait for, or fail on, the forty-odd others the suite ships with.
//
// Not wrapped in the suite's `addDidv11Implementations`, which re-runs an input
// with its context rewritten to DID 1.1. That is a document we do not produce,
// and no DID 1.1 claim is made.
const CONFIGURED = {
  "did-core-properties": "did-spec",
  "did-production": "did-production",
};

// What every produced DID must have been through, in each suite that carries
// it. These are the rules the claim leans on. §5.4 Services is deliberately
// absent: it passes without exercising anything over a document with no service.
const REQUIRED = {
  "did-core-properties": [
    "5.1.1 DID Subject",
    "5.2 Verification Methods",
    "5.3.1 Authentication",
    "5.3.2 Assertion",
    "7.3 Metadata Structure",
  ],
  "did-production": [
    "6.1 Production and Consumption",
    "6.3.1 JSON-LD Production",
  ],
};

// The rules the canary breaks, so that the suite is shown to catch each.
const CANARY = [
  {
    broken: "a verification method's controller is not a DID",
    expectFailing: "The value of the controller property MUST be a string that conforms",
  },
  {
    broken: "a verification method's JWK carries a private member",
    expectFailing: 'The map MUST NOT contain "d"',
  },
];

const input = JSON.parse(readFileSync(inputPath, "utf8"));
if (!Array.isArray(input.dids) || input.dids.length === 0) {
  console.error(`${inputPath} lists no DIDs`);
  process.exit(1);
}

/** Install `implementation` where the suite looks, then run the two suites. */
function run(implementation, label) {
  writeFileSync(path.join(suites, "implementations", IMPLEMENTATION), JSON.stringify(implementation));

  for (const [suite, name] of Object.entries(CONFIGURED)) {
    writeFileSync(
      path.join(suites, suite, "default.js"),
      `module.exports = {\n  name: '${name}',\n  didMethods: [require('../implementations/${IMPLEMENTATION}')],\n};\n`,
    );
  }

  const out = path.join(mkdtempSync(path.join(tmpdir(), "did-suite-")), "results.json");
  const jest = spawnSync(
    process.execPath,
    [
      path.join(server, "node_modules/jest/bin/jest.js"),
      "--ci",
      "--json",
      `--outputFile=${out}`,
      ...Object.keys(CONFIGURED).map((suite) => `${suite}.spec`),
    ],
    { cwd: server, encoding: "utf8" },
  );

  let results;
  try {
    results = JSON.parse(readFileSync(out, "utf8"));
  } catch {
    console.error(`${label}: jest wrote no results (exit ${jest.status})\n${jest.stderr}`);
    process.exit(1);
  }
  return { results, stderr: jest.stderr };
}

const assertions = (results) =>
  results.testResults.flatMap((file) =>
    file.assertionResults.map((a) => ({ ...a, file: path.basename(file.name) })),
  );

const problems = [];

// ── The documents as produced ────────────────────────────────────────────
const control = run(input, "control");
const all = assertions(control.results);
const failed = all.filter((a) => a.status === "failed");
const skipped = all.filter((a) => a.status !== "passed" && a.status !== "failed");

console.log(
  `${all.length} assertions over ${input.dids.length} DID documents: ` +
    `${all.length - failed.length - skipped.length} passed, ${failed.length} failed, ${skipped.length} skipped`,
);

for (const a of failed) {
  problems.push(`FAILED  ${a.fullName}\n          ${(a.failureMessages[0] ?? "").split("\n")[0]}`);
}
for (const a of skipped) problems.push(`NOT RUN (${a.status})  ${a.fullName}`);
if (control.results.numRuntimeErrorTestSuites > 0) {
  problems.push(`${control.results.numRuntimeErrorTestSuites} suite(s) crashed before running\n${control.stderr}`);
}

for (const [suite, rules] of Object.entries(REQUIRED)) {
  for (const did of input.dids) {
    for (const rule of rules) {
      const ran = all.some(
        (a) =>
          a.file === `${suite}.spec.js` &&
          a.status === "passed" &&
          a.ancestorTitles.includes(did) &&
          a.title.startsWith(rule),
      );
      if (!ran) problems.push(`NEVER RAN  ${suite}: "${rule}" for ${did}`);
    }
  }
}

// ── The same documents, broken ───────────────────────────────────────────
const broken = structuredClone(input);
const target = broken[broken.dids[0]].didDocumentDataModel.properties.verificationMethod[0];
target.controller = "this-is-not-a-did";
target.publicKeyJwk.d = "AAAA";

const canary = run(broken, "canary");
const caught = assertions(canary.results).filter((a) => a.status === "failed");

for (const { broken: what, expectFailing } of CANARY) {
  if (!caught.some((a) => a.title.includes(expectFailing))) {
    problems.push(`CANARY  the suite did not fail when ${what}; it no longer discriminates`);
  }
}
console.log(`canary: ${caught.length} assertions failed on the broken copy`);

if (problems.length > 0) {
  console.error(`\n${problems.join("\n")}\n\n${problems.length} problem(s).`);
  process.exit(1);
}
console.log("The produced DID documents pass the W3C DID test suite, and the suite can fail.");
