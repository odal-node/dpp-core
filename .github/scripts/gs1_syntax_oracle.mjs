/**
 * Judge our Digital Link corpus with GS1's own syntax tooling.
 *
 * The corpus is produced by `dpp-digital-link`'s `gs1_oracle_corpus` test and
 * carries, for every URI, the verdict *our* parser reached. This script asks the
 * GS1 Barcode Syntax Engine the same question and fails on any disagreement.
 *
 * Both directions are failures, and they fail differently:
 *
 *   we accept, GS1 rejects   we emit links that are not valid GS1, onto products
 *   we reject, GS1 accepts   we refuse links a conformant partner may send us —
 *                            the quiet one, which loses interoperability with no
 *                            error ever logged
 *
 * The engine implements the GS1 Barcode Syntax Dictionary; it is the published
 * tooling, not a second parser we wrote. That is the entire point: our own tests
 * were written by whoever wrote the parser, so they agree with it including
 * where it is wrong.
 *
 * **This proves syntax, not semantics.** It says nothing about whether a GTIN is
 * allocated to anyone or whether a resolver answers, and it does not support the
 * phrase "GS1-certified". The engine version is printed so the run is
 * attributable to a specific build on a specific date.
 *
 * Exits non-zero listing every disagreement, not just the first.
 */

import { readFileSync } from "node:fs";
import { GS1encoder } from "gs1encoder";

const corpusPath = process.argv[2] ?? "target/gs1-oracle/corpus.jsonl";

const entries = readFileSync(corpusPath, "utf8")
  .split("\n")
  .filter((line) => line.trim() !== "")
  .map((line) => JSON.parse(line));

if (entries.length === 0) {
  console.error(`no corpus entries in ${corpusPath}`);
  process.exit(1);
}

const engine = new GS1encoder();
await engine.init();
console.log(`GS1 Barcode Syntax Engine build: ${engine.version}`);
console.log(`corpus: ${entries.length} entries from ${corpusPath}\n`);

const disagreements = [];
let checked = 0;

for (const { uri, accepted, note } of entries) {
  let oracleAccepted;
  let detail = "";
  try {
    engine.dataStr = uri;
    oracleAccepted = true;
  } catch (err) {
    oracleAccepted = false;
    detail = String(err?.message ?? err).split("\n")[0];
  }

  checked += 1;
  if (oracleAccepted !== accepted) {
    disagreements.push({ uri, note, ours: accepted, gs1: oracleAccepted, detail });
  }
}

engine.free();

for (const d of disagreements) {
  const direction = d.ours
    ? "we ACCEPT, GS1 REJECTS — we would emit an invalid link"
    : "we REJECT, GS1 ACCEPTS — we would refuse a conformant partner's link";
  console.error(`FAIL [${d.note}] ${d.uri}`);
  console.error(`  ${direction}`);
  if (d.detail) console.error(`  engine: ${d.detail}`);
}

if (disagreements.length > 0) {
  console.error(
    `\n${disagreements.length} of ${checked} entries disagree with the GS1 Barcode Syntax Engine.`,
  );
  process.exit(1);
}

console.log(`All ${checked} entries agree with the GS1 Barcode Syntax Engine.`);
