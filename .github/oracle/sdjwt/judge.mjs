/**
 * Judge the SD-JWTs this workspace emits with an independent implementation of
 * RFC 9901.
 *
 * `corpus.json` comes from
 * `EMIT_SDJWT_CORPUS=1 cargo test -p dpp-crypto --lib sd_jwt::oracle_tests::emit`.
 * Four groups, four questions:
 *
 *   issued, presented  Tokens this crate built from a claim tree, with the claims a
 *                      verifier must see, worked out without this crate's code. The
 *                      library must arrive at exactly those claims, so a digest or
 *                      a placeholder this crate builds wrongly shows as a claim that
 *                      is missing, or that should not be there.
 *   adversarial        Tokens each broken in one place and re-signed, so structure is
 *                      the only fault, plus controls that must be accepted. Both
 *                      implementations judge them. They must give the same verdict
 *                      and, where both accept, the same claims. A difference is
 *                      either fixed in this crate or listed below with the reason
 *                      and the side the RFC supports. A listed difference that no
 *                      longer exists fails, so the list cannot go stale.
 *   rfc                The RFC's own tokens, as vendored in this repository. The
 *                      library verifies the RFC's ES256 signature on each, with the
 *                      key the RFC prints in Appendix A.5, and the Key Binding JWT's
 *                      `sd_hash` where there is one. Either covers every byte, so a
 *                      transcription slip in a vendored string fails here.
 *
 * The hiding of claim order (clause 4.2.4.1) is judged too, in the one way it can
 * be: the original order cannot be seen, but whether each `_sd` array is sorted can.
 * A sorted array is a function of the salted digests alone, so it cannot carry the
 * order the claims were in, and sorting is the method the clause names. Every `_sd`
 * array in every token this crate issues must be strictly ascending, and so must
 * the RFC's own, which shows the convention is theirs and not ours.
 *
 * What the signature check is: Node's `crypto`, in `lib.mjs`, doing the Ed25519
 * arithmetic (OpenSSL, which shares nothing with this crate's `ed25519-dalek`). The
 * library builds the JWS signing input and calls back for the check, so it frames
 * the signature and Node verifies it.
 *
 * Negative controls run first, because a judge that cannot refuse proves nothing.
 * The script then reports every failure and exits non-zero if there is one.
 */

import { readFileSync } from "node:fs";
import { generateKeyPairSync } from "node:crypto";
import {
  ascending,
  judge,
  libraryVersion,
  same,
  sdArrays,
  verifyingInstance,
} from "./lib.mjs";

const corpusPath = process.argv[2] ?? "target/sdjwt-oracle/corpus.json";

/** RFC 9901 Appendix A.5, verbatim: the key that validates the Issuer signatures. */
const RFC_ISSUER_KEY = {
  kty: "EC",
  crv: "P-256",
  x: "b28d4MwZMjw8-00CG4xfnn9SLMVMM19SlqZpVb_uNtQ",
  y: "Xv5zWwuoaTgdS6hV43yI6gBwTnjukmFQQnJ_kCxzqk8",
};

/**
 * Where the library and this crate give different verdicts, and why that is not a
 * defect here. Keyed by case id. Each entry names which side the RFC supports.
 * Anything not listed that differs fails the run, and so does an entry that no
 * longer differs.
 */
const KNOWN_DISAGREEMENTS = {
  "adversarial/reject/jwt-payload-is-an-array": {
    ours: "refuse",
    theirs: "accept",
    supports: "ours",
    reason:
      "Clause 4.1: the payload of an SD-JWT is a JSON object. The library reads an array payload; this crate refuses it. The library is the lenient one.",
  },
  "adversarial/edge/hash-algorithm-sha-512": {
    ours: "refuse",
    theirs: "accept",
    supports: "both",
    reason:
      "Clause 4.1.1 allows any IANA hash and clause 7.1 step 2.d leaves the choice to the verifier's policy. This crate implements SHA-256 only, on purpose. The judge's hasher computes SHA-512 so that the library can accept it.",
  },
  "adversarial/edge/nested-200-deep": {
    ours: "refuse",
    theirs: "accept",
    supports: "both",
    reason:
      "No clause limits nesting. serde_json refuses a document nested more than 127 deep, and a receiver sets its own limit.",
  },
};

const failures = [];
const fail = (id, why) => failures.push(`${id}: ${why}`);
const verdict = (v) => (v.accepts ? "accept" : "refuse");

const corpus = JSON.parse(readFileSync(corpusPath, "utf8"));
const byId = new Map(corpus.cases.map((c) => [c.id, c]));
const ed25519 = verifyingInstance(corpus.issuerJwk, "EdDSA");

console.log(`independent implementation: @sd-jwt/core ${libraryVersion}`);
console.log(`node ${process.version}, ${corpus.cases.length} cases in ${corpusPath}`);

// ---- Negative controls ------------------------------------------------------

async function negativeControls() {
  const problems = [];
  const valid = byId.get("adversarial/control/valid")?.token;
  if (!valid) return ["the corpus has no control/valid case"];

  if (!(await judge(ed25519, valid)).accepts) {
    problems.push("control/valid is refused, so nothing it refuses means anything");
  }

  const [jwt, ...rest] = valid.split("~");
  const [h, p, s] = jwt.split(".");
  const flip = (c) => (c === "A" ? "B" : "A");
  const flipped = [[h, p, flip(s[0]) + s.slice(1)].join("."), ...rest].join("~");
  if ((await judge(ed25519, flipped)).accepts) {
    problems.push("a token with one signature byte changed was accepted");
  }

  const stranger = verifyingInstance(
    generateKeyPairSync("ed25519").publicKey.export({ format: "jwk" }),
    "EdDSA",
  );
  if ((await judge(stranger, valid)).accepts) {
    problems.push("a token was accepted under a key that did not sign it");
  }

  const [, first] = valid.split("~");
  const tampered = valid.replace(first, flip(first[0]) + first.slice(1));
  if ((await judge(ed25519, tampered)).accepts) {
    problems.push("a token with a changed Disclosure was accepted");
  }

  // The order check has to be able to fail, on a real token and not only on a list.
  if (ascending(["b", "a"]) || ascending(["a", "a"]) || !ascending(["a", "b"]) || !ascending([])) {
    problems.push("the ascending check is wrong on a plain list");
  }
  if (sdArrays(valid).length === 0) problems.push("no _sd array is found in a token that has some");
  // Sorted descending rather than reversed: reversing an array that was never
  // sorted can leave it ascending, and then an issuer that stopped sorting would
  // fail this control instead of the order check, for the wrong reason.
  const body = JSON.parse(Buffer.from(p, "base64url"));
  body._sd.sort().reverse();
  const descending = [[h, Buffer.from(JSON.stringify(body)).toString("base64url"), s].join("."), ...rest].join("~");
  if (!sdArrays(descending).some((a) => a.length > 1 && !ascending(a))) {
    problems.push("a token whose _sd array is in descending order is not flagged");
  }

  if (!same({ a: 1, b: [1, { c: 2 }] }, { b: [1, { c: 2 }], a: 1 })) {
    problems.push("the comparison treats key order as a difference");
  }
  for (const [a, b] of [
    [{ a: 1 }, { a: 2 }],
    [{ a: 1 }, { a: 1, b: 1 }],
    [{ a: [1, 2] }, { a: [2, 1] }],
    [{ a: "1" }, { a: 1 }],
  ]) {
    if (same(a, b)) problems.push(`the comparison cannot tell ${JSON.stringify(a)} from ${JSON.stringify(b)}`);
  }
  return problems;
}

const broken = await negativeControls();
if (broken.length) {
  console.error("NEGATIVE CONTROLS FAILED; the judge cannot be trusted:");
  for (const b of broken) console.error(`  ${b}`);
  process.exit(2);
}
console.log("negative controls: the judge refuses what it must");

// ---- issued, presented ------------------------------------------------------

const counts = {};
const tally = (group) => (counts[group] = (counts[group] ?? 0) + 1);

/** Every `_sd` array in the token, counted, and each must be strictly ascending. */
const order = { ours: { tokens: 0, arrays: 0 }, rfc: { tokens: 0, arrays: 0 } };
function checkOrder(whose, id, token) {
  order[whose].tokens++;
  for (const digests of sdArrays(token)) {
    if (digests.length < 2) continue;
    order[whose].arrays++;
    if (!ascending(digests)) fail(id, "an _sd array is not in ascending order, so its order may follow the claims'");
  }
}

for (const c of corpus.cases.filter((c) => c.group === "issued" || c.group === "presented")) {
  tally(c.group);
  checkOrder("ours", c.id, c.token);
  const theirs = await judge(ed25519, c.token);
  if (!c.ours.accepts) fail(c.id, `this crate refuses its own token: ${c.ours.error}`);
  if (!theirs.accepts) {
    fail(c.id, `the library refuses it: ${theirs.error}`);
  } else if (!same(theirs.payload, c.expected)) {
    fail(
      c.id,
      `the library reads ${JSON.stringify(theirs.payload)}, a verifier must see ${JSON.stringify(c.expected)}`,
    );
  }
}

// ---- adversarial ------------------------------------------------------------

const seenDisagreements = new Set();
for (const c of corpus.cases.filter((c) => c.group === "adversarial")) {
  tally("adversarial");
  const theirs = await judge(ed25519, c.token);
  const known = KNOWN_DISAGREEMENTS[c.id];
  const isControl = c.id.startsWith("adversarial/control/");

  if (isControl && !(c.ours.accepts && theirs.accepts)) {
    fail(c.id, `a control must be accepted by both (ours ${verdict(c.ours)}, library ${verdict(theirs)})`);
    continue;
  }

  const differs =
    c.ours.accepts !== theirs.accepts ||
    (c.ours.accepts && !same(c.ours.payload, theirs.payload));
  if (!differs) {
    if (known) fail(c.id, "listed as a disagreement, but the two now agree: remove the entry");
    continue;
  }
  if (!known) {
    fail(
      c.id,
      c.ours.accepts !== theirs.accepts
        ? `this crate ${verdict(c.ours)}s, the library ${verdict(theirs)}s (${c.ours.error ?? theirs.error})`
        : `both accept, but read ${JSON.stringify(c.ours.payload)} and ${JSON.stringify(theirs.payload)}`,
    );
    continue;
  }
  seenDisagreements.add(c.id);
  if (known.ours !== verdict(c.ours) || known.theirs !== verdict(theirs)) {
    fail(c.id, `the listed disagreement is ours ${known.ours} / library ${known.theirs}, now ours ${verdict(c.ours)} / library ${verdict(theirs)}`);
  }
}
for (const id of Object.keys(KNOWN_DISAGREEMENTS)) {
  if (!byId.has(id)) fail(id, "listed as a disagreement, but the corpus has no such case");
}

// ---- RFC examples -----------------------------------------------------------

const es256 = verifyingInstance(RFC_ISSUER_KEY, "ES256");

/**
 * The claims of a compact JWT, read without checking anything, or `{}` when it
 * cannot be read: the library is the one to say so, in the report.
 */
function claimsOf(jwt) {
  try {
    return JSON.parse(Buffer.from(jwt.split(".")[1], "base64url"));
  } catch {
    return {};
  }
}

for (const c of corpus.cases.filter((c) => c.group === "rfc")) {
  tally("rfc");
  checkOrder("rfc", c.id, c.token);
  const [jwt, ...tail] = c.token.split("~");
  const kb = tail.at(-1) ? claimsOf(tail.at(-1)) : null;
  // The examples are old; judge them as of when they were made.
  const options = { currentDate: kb?.iat ?? claimsOf(jwt).iat };
  if (kb) {
    options.keyBindingNonce = kb.nonce;
    options.expectedKeyBindingAudience = kb.aud;
  }
  const theirs = await judge(es256, c.token, options);
  if (!theirs.accepts) {
    fail(c.id, `the library does not verify the RFC's own token: ${theirs.error}`);
    continue;
  }
  if (c.printed && !same(theirs.payload, c.printed)) {
    fail(c.id, "the library's reading differs from the payload the RFC prints");
  }
  if (!c.ours.accepts) {
    fail(c.id, `this crate refuses it: ${c.ours.error}`);
  } else if (!same(c.ours.payload, theirs.payload)) {
    fail(c.id, "this crate and the library read different claims");
  }
}

// ---- Report -----------------------------------------------------------------

console.log(
  Object.entries(counts)
    .map(([group, n]) => `${group}: ${n}`)
    .join(", "),
);
console.log(
  `claim order (clause 4.2.4.1): ${order.ours.arrays} _sd arrays of two or more digests in ` +
    `${order.ours.tokens} tokens this crate issued, ${order.rfc.arrays} in the RFC's ` +
    `${order.rfc.tokens} vendored tokens; each strictly ascending`,
);
console.log(`known disagreements, each argued in the script: ${seenDisagreements.size}`);
for (const id of seenDisagreements) {
  const k = KNOWN_DISAGREEMENTS[id];
  console.log(`  ${id}\n    ours ${k.ours}, library ${k.theirs}; RFC supports ${k.supports}. ${k.reason}`);
}

if (failures.length) {
  console.error(`\n${failures.length} FAILURE(S):`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}
console.log("every case agrees");
