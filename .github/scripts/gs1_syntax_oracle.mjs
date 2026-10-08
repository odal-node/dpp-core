/**
 * Judge our Digital Link corpora with GS1's own syntax tooling.
 *
 * Section 2 of GS1 Digital Link URI Syntax names the GS1 Barcode Syntax Resource
 * as a tool for confirming conformance. The Resource is a dictionary of every AI,
 * a set of linters, and the GS1 Barcode Syntax Engine that runs them, which is
 * what this script drives. Given a URI that begins `http://` or `https://` the
 * engine parses the Digital Link and validates what it finds: the AIs, their
 * lengths and character sets, the check digits, the qualifier order. It is the
 * published tooling, not a second parser we wrote, which is the entire point: our
 * own tests were written by whoever wrote the parser, so they agree with it
 * including where it is wrong.
 *
 * Two corpora, two kinds of entry:
 *
 *   `corpus.jsonl`  from `gs1_oracle_corpus`: every link this crate builds, and a
 *                   few it must refuse, each with the verdict *our* parser
 *                   reached. The engine must agree, in both directions:
 *
 *                     we accept, GS1 rejects   we emit links that are not valid
 *                                              GS1, onto products
 *                     we reject, GS1 accepts   we refuse links a conformant
 *                                              partner may send us — the quiet
 *                                              one, which loses interoperability
 *                                              with no error ever logged
 *
 *   `rules.jsonl`   from `gs1_syntax_rules_corpus`: one URI for each rule of the
 *                   section 4 grammar. Each carries `grammar`, the verdict the
 *                   grammar gives it, and `engineDiffers`, a reason, present
 *                   exactly where the engine's verdict is known not to be the
 *                   grammar's (it is lenient about some things, and refuses some
 *                   the grammar allows). The engine must give the grammar's
 *                   verdict, or the opposite where a reason says so. A reason
 *                   that has stopped being true fails as surely as a difference
 *                   nobody wrote down, so the list of the engine's departures
 *                   cannot go stale.
 *
 * An entry marked `legacy` is also judged with the engine's mode for zero-
 * suppressed GTINs, which must accept it: that is GS1's own statement that the
 * form is legacy and not wrong, and it is what this crate's reader does.
 *
 * **This proves syntax, not semantics.** It says nothing about whether a GTIN is
 * allocated to anyone or whether a resolver answers, and it does not support the
 * phrase "GS1-certified". The engine version is printed so the run is
 * attributable to a specific build on a specific date.
 *
 * Exits non-zero listing every failure, not just the first.
 */

import { readFileSync } from "node:fs";
import { GS1encoder } from "gs1encoder";

const paths =
  process.argv.length > 2
    ? process.argv.slice(2)
    : ["target/gs1-oracle/corpus.jsonl", "target/gs1-oracle/rules.jsonl"];

const strict = new GS1encoder();
await strict.init();
const legacy = new GS1encoder();
await legacy.init();
legacy.permitZeroSuppressedGTINinDLuris = true;
console.log(`GS1 Barcode Syntax Engine build: ${strict.version}`);

/** The engine's verdict on one URI, and its reason when it refuses. */
function judge(engine, uri) {
  try {
    engine.dataStr = uri;
    return { accepted: true, detail: "" };
  } catch (err) {
    return { accepted: false, detail: String(err?.message ?? err).split("\n")[0] };
  }
}

const failures = [];
const oursDiffers = new Map();
const engineDiffers = new Map();
let checked = 0;

for (const path of paths) {
  const entries = readFileSync(path, "utf8")
    .split("\n")
    .filter((line) => line.trim() !== "")
    .map((line) => JSON.parse(line));
  if (entries.length === 0) {
    console.error(`no corpus entries in ${path}`);
    process.exit(1);
  }
  console.log(`corpus: ${entries.length} entries from ${path}`);

  for (const entry of entries) {
    const { uri, accepted } = entry;
    const label = entry.rule ?? entry.note;
    const engine = judge(strict, uri);
    checked += 1;

    if (entry.grammar === undefined) {
      // The first corpus: the engine must agree with our parser.
      if (engine.accepted !== accepted) {
        failures.push({
          label,
          uri,
          what: accepted
            ? "we ACCEPT, GS1 REJECTS — we would emit an invalid link"
            : "we REJECT, GS1 ACCEPTS — we would refuse a conformant partner's link",
          detail: engine.detail,
        });
      }
      continue;
    }

    // The rules corpus: both tools are pinned to the grammar's verdict, except
    // where a reason says otherwise.
    const expectedOurs = entry.oursDiffers ? !entry.grammar : entry.grammar;
    if (accepted !== expectedOurs) {
      failures.push({
        label,
        uri,
        what: `the corpus says our parser ${accepted ? "accepts" : "rejects"} this, which is not what its own annotations imply`,
        detail: "",
      });
    }
    const expectedEngine = entry.engineDiffers ? !entry.grammar : entry.grammar;
    if (engine.accepted !== expectedEngine) {
      failures.push({
        label,
        uri,
        what: entry.engineDiffers
          ? `the engine now ${engine.accepted ? "accepts" : "rejects"} this: the recorded difference ("${entry.engineDiffers}") no longer holds`
          : `the grammar says ${entry.grammar ? "valid" : "invalid"} and the engine ${engine.accepted ? "accepts" : "rejects"} it, with no difference recorded`,
        detail: engine.detail,
      });
    }
    if (entry.legacy && !judge(legacy, uri).accepted) {
      failures.push({
        label,
        uri,
        what: "the engine's legacy mode for zero-suppressed GTINs refuses a form this reader accepts as legacy",
        detail: judge(legacy, uri).detail,
      });
    }
    if (entry.oursDiffers) oursDiffers.set(entry.oursDiffers, (oursDiffers.get(entry.oursDiffers) ?? 0) + 1);
    if (entry.engineDiffers) engineDiffers.set(entry.engineDiffers, (engineDiffers.get(entry.engineDiffers) ?? 0) + 1);
  }
}

strict.free();
legacy.free();

for (const f of failures) {
  console.error(`FAIL [${f.label}] ${f.uri}`);
  console.error(`  ${f.what}`);
  if (f.detail) console.error(`  engine: ${f.detail}`);
}

console.log("\nWhere this crate's reader differs from the grammar, by recorded reason:");
for (const [reason, n] of oursDiffers) console.log(`  ${n} × ${reason}`);
console.log("\nWhere the engine differs from the grammar, by recorded reason:");
for (const [reason, n] of engineDiffers) console.log(`  ${n} × ${reason}`);

if (failures.length > 0) {
  console.error(`\n${failures.length} failure(s) in ${checked} entries.`);
  process.exit(1);
}

console.log(`\nAll ${checked} entries agree with the GS1 Barcode Syntax Engine.`);
