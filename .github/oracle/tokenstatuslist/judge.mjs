/**
 * Judges what `dpp-vc::token_status_list` accepts and refuses with the independent
 * implementation, and the draft's own test vectors with it too.
 *
 * Reads `target/status-list-oracle/corpus.json`, which
 * `EMIT_STATUS_LIST_CORPUS=1 cargo test -p dpp-tests --test status_list_oracle_corpus`
 * writes: Status List Tokens this workspace's code verifies or refuses, each with the
 * key it is signed under, the `uri` of the reference it is read for, and the entries
 * to read.
 *
 * Per token, the signature is checked by Node's `crypto` (see `lib.mjs`), and then
 * the library does what the draft's section 8.3 asks of a Relying Party after that:
 * it reads the token (`typ`, the claims' shapes, the list's `bits` and `lst`), checks
 * its claims against the `uri` and the time (`sub`, `exp`), and reads the status at
 * each index. A token this crate accepts must come out of that with the entries the
 * corpus gives; a token this crate refuses must be refused somewhere on the way.
 *
 * The draft's Appendix C vectors are read the same way, as a control: they show the
 * library and this crate agree with the draft and not only with each other.
 */

import path from "node:path";

import { draftVectors, library, libraryVersion, outDir, readJson, signatureHolds } from "./lib.mjs";

const { StatusList, decodeStatusListJwt, verifyStatusListJwtClaims } = library;

/**
 * Tokens this crate refuses and the library does not, each with the reason, so a
 * difference of strictness is written down and not found again. Empty on purpose:
 * an entry is added only after the clause has been read.
 */
const KNOWN_LENIENCIES = {};

const corpus = readJson(path.join(outDir, "corpus.json"));
const failures = [];
const fail = (name, message) => failures.push(`${name}: ${message}`);

/** What the library makes of one token: `{ values }` for the entries asked for, or `{ refused }` and why. */
function read(testcase) {
  const [header, payload, signature] = testcase.token.split(".");
  if (!signatureHolds(testcase.alg, testcase.jwk, `${header}.${payload}`, signature ?? "")) {
    return { refused: "the signature does not hold (Node)" };
  }
  try {
    const decoded = decodeStatusListJwt(testcase.token);
    if (decoded.header.alg !== testcase.alg) return { refused: `alg is ${decoded.header.alg}` };
    verifyStatusListJwtClaims(decoded.payload, {
      uri: testcase.uri,
      now: new Date(corpus.now * 1000),
      // This crate compares exactly. The library's default leeway of 30 seconds is a
      // policy, and a token that sits inside it would be a difference of policy.
      skewSeconds: 0,
      requiredClaims: ["sub", "iat"],
    });
    const list = decoded.statusList;
    return { values: testcase.idxs.map((idx) => list.getStatus(idx)) };
  } catch (error) {
    return { refused: `${error.constructor.name}: ${error.message}` };
  }
}

for (const testcase of corpus.cases) {
  const verdict = read(testcase);
  // Said for every token, so a refusal for the wrong reason can be seen and not only
  // counted: a library that refused everything for one reason would pass the refusals.
  console.log(
    `${testcase.accepts ? "reads  " : "refuses"} ${testcase.name}: ` +
      (verdict.refused ? `the library refuses (${verdict.refused.slice(0, 110)})` : "the library reads it"),
  );
  if (testcase.accepts) {
    if (verdict.refused) fail(testcase.name, `this crate accepts it and the library refuses it: ${verdict.refused}`);
    else if (JSON.stringify(verdict.values) !== JSON.stringify(testcase.expect)) {
      fail(testcase.name, `the library reads ${JSON.stringify(verdict.values)}, this crate ${JSON.stringify(testcase.expect)}`);
    }
  } else if (!verdict.refused && !(testcase.name in KNOWN_LENIENCIES)) {
    fail(testcase.name, "this crate refuses it and the library reads it");
  }
}

for (const vector of draftVectors()) {
  const bytes = new Uint8Array(Buffer.from(vector.lst, "base64url"));
  const list = StatusList.decompressStatusListFromBytes(bytes, vector.bits);
  const name = `draft ${vector.name}`;
  if (list.totalStatuses !== 2 ** 20) fail(name, `${list.totalStatuses} entries, not 2^20`);
  for (const [idx, value] of vector.statuses) {
    if (list.getStatus(idx) !== value) fail(name, `entry ${idx} is ${list.getStatus(idx)}, the draft says ${value}`);
  }
  let set = 0;
  for (let idx = 0; idx < list.totalStatuses; idx++) if (list.getStatus(idx) !== 0) set++;
  const listed = vector.statuses.filter(([, value]) => value !== 0).length;
  if (set !== listed) fail(name, `${set} entries are not zero, the draft lists ${listed}`);
}

const accepted = corpus.cases.filter((c) => c.accepts).length;
console.log(
  `@owf/token-status-list ${libraryVersion}: ${corpus.cases.length} tokens (${accepted} accepted, ` +
    `${corpus.cases.length - accepted} refused by this crate) and ${draftVectors().length} draft vectors judged`,
);
if (failures.length > 0) {
  console.error(`${failures.length} disagreements:\n${failures.join("\n")}`);
  process.exit(1);
}
