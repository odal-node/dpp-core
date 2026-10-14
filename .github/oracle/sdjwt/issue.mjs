/**
 * The other direction: tokens an independent implementation issues, for this crate
 * to read.
 *
 * `judge.mjs` asks whether the library reads what this crate wrote. The RFC's own
 * examples are one issuer's output and this crate's tokens are another's, so
 * neither says what a conformant issuer that is not us produces. This script
 * issues with the library: object properties, nested and recursive ones, array
 * elements in every position, arrays of objects and of arrays, decoys at object and
 * array level, a Key Binding JWT, 120 seeded random trees, and the library's own
 * JSON serialisations, which this crate does not read and must refuse cleanly.
 *
 * For every token the library verifies it first, and what it says the claims are is
 * the `expected` value. This crate has to arrive at the same claims
 * (`sd_jwt::oracle_tests::reverse`, run with `CHECK_SDJWT_ORACLE_ISSUED`). If the
 * library cannot read its own token, that is reported here and the run fails,
 * because a case nobody judged would otherwise just be missing.
 *
 * The key, the salts and the shapes all come from fixed seeds, so the file is the
 * same on every run. Two runs that differ mean the library changed.
 *
 * Writes `target/sdjwt-oracle/issued.json`:
 *   { publicKey: <base64url raw Ed25519 key>, cases: [{ id, verdict, token,
 *     expected?, keyBinding?, challenge? }] }, where `challenge` is the nonce,
 *     audience and instant a bound token is verified against
 */

import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  SDJwtInstance,
  ascending,
  b64u,
  ed25519FromSeed,
  hasher,
  kbVerifier,
  libraryVersion,
  prng,
  sdArrays,
  verifierFor,
} from "./lib.mjs";

const outPath = process.argv[2] ?? "target/sdjwt-oracle/issued.json";

const issuer = ed25519FromSeed("sdjwt-oracle issuer, reverse direction");
const holder = ed25519FromSeed("sdjwt-oracle holder, reverse direction");
const rng = prng(0x5d3a_9001);

const sd = new SDJwtInstance({
  hasher,
  hashAlg: "sha-256",
  signAlg: "EdDSA",
  signer: (data) => issuer.sign(data),
  verifier: verifierFor(issuer.publicJwk),
  kbSignAlg: "EdDSA",
  kbSigner: (data) => holder.sign(data),
  kbVerifier,
  // Fresh bytes from the seeded generator, so a salt is never reused within a run
  // and never differs between runs.
  saltGenerator: (n) => b64u(Uint8Array.from({ length: n }, () => Math.floor(rng() * 256))),
});

const KB = { iat: 1_700_000_000, aud: "https://verifier.example" };

const cases = [];
const problems = [];

/**
 * How many Disclosures `frame` asks the library to issue: one for every entry of
 * every `_sd`, at any depth. Decoys (`_sd_decoy`) have none.
 */
function disclosuresAskedFor(frame) {
  if (frame === null || typeof frame !== "object") return 0;
  return Object.entries(frame).reduce(
    (total, [key, value]) =>
      total + (key === "_sd" ? value.length : key === "_sd_decoy" ? 0 : disclosuresAskedFor(value)),
    0,
  );
}

/**
 * Whether the library issued every Disclosure `frame` asked for. A frame it
 * does not understand is not refused: it issues fewer, or none, and the token is
 * still valid. `@sd-jwt/core` does exactly that with an array index written as a
 * string, so a case meant to hide an array element hid nothing and passed, because
 * what the library reads back is what a case expects. Such a case proves nothing,
 * so it fails the run instead.
 */
function checkIssued(id, frame, issued) {
  const asked = disclosuresAskedFor(frame);
  const made = issued.split("~").length - 2;
  if (made === asked) return true;
  problems.push(`${id}: the frame asks for ${asked} Disclosures and the library issued ${made}, so the case does not hide what it says it hides`);
  return false;
}

/**
 * Issue `payload` under `frame`, present it under `presentation` (everything
 * when omitted), have the library read the result, and record both.
 */
async function add(id, payload, frame, presentation, { keyBinding = false } = {}) {
  try {
    const claims = { iss: "https://issuer.example", ...payload };
    if (keyBinding) claims.cnf = { jwk: holder.publicJwk };
    const issued = await sd.issue(claims, frame);
    if (!checkIssued(id, frame, issued)) return;
    const nonce = `nonce-${id}`;
    const token =
      presentation === undefined && !keyBinding
        ? issued
        : await sd.present(issued, presentation ?? {}, keyBinding ? { kb: { payload: { ...KB, nonce } } } : undefined);
    const options = keyBinding
      ? { keyBindingNonce: nonce, currentDate: KB.iat, expectedKeyBindingAudience: KB.aud }
      : {};
    const { payload: expected } = await sd.verify(token, options);
    // A bound token carries the challenge it answers, so the reader is held to
    // the same nonce, audience and instant the library verified it with.
    const challenge = keyBinding ? { challenge: { nonce, aud: KB.aud, now: KB.iat, window: 0 } } : {};
    cases.push({ id, verdict: "accept", token, expected, keyBinding, ...challenge });
  } catch (e) {
    problems.push(`${id}: the library cannot issue or read this: ${e?.message ?? e}`);
  }
}

// ---- Handwritten shapes -----------------------------------------------------

const record = { name: "Alice", age: 30, active: true, nothing: null, ratio: 1.5, tags: ["a", "b"], meta: { k: 1 } };

await add("flat/nothing-hidden", record, undefined);
await add("flat/all-properties-hidden-all-shown", record, { _sd: Object.keys(record) });
await add("flat/all-properties-hidden-none-shown", record, { _sd: Object.keys(record) }, {});
await add("flat/some-hidden-some-shown", record, { _sd: ["name", "ratio", "meta"] }, { name: true });
await add("flat/non-ascii-names", { "naïve": 1, "日本語": "x", "emoji😀": [1] }, { _sd: ["naïve", "日本語"] }, { 日本語: true });

const nested = { person: { name: "Alice", address: { city: "Berlin", zip: "10115" } }, other: 1 };
await add("nested/inner-property-only", nested, { person: { _sd: ["name"] } }, { person: { name: true } });
await add("nested/object-hidden-inner-visible", nested, { _sd: ["person"] }, { person: true });
await add(
  "recursive/object-and-its-members-hidden",
  nested,
  { _sd: ["person"], person: { _sd: ["name", "address"], address: { _sd: ["city", "zip"] } } },
  { person: { name: true, address: { city: true } } },
);
await add(
  "recursive/all-revealed",
  nested,
  { _sd: ["person"], person: { _sd: ["name", "address"], address: { _sd: ["city", "zip"] } } },
  { person: { name: true, address: { city: true, zip: true } } },
);
await add(
  "recursive/only-the-parent-revealed",
  nested,
  { _sd: ["person"], person: { _sd: ["name", "address"], address: { _sd: ["city"] } } },
  { person: true },
);

// An index into an array is a number in `_sd`. A string is ignored, and nothing is hidden.
const list = { list: ["first", "middle", "last"] };
for (const [name, index] of [["first", 0], ["middle", 1], ["last", 2]]) {
  await add(`array/${name}-element-hidden-shown`, list, { list: { _sd: [index] } }, { list: { [index]: true } });
  await add(`array/${name}-element-hidden-withheld`, list, { list: { _sd: [index] } }, { list: {} });
}
await add("array/every-element-hidden-some-shown", list, { list: { _sd: [0, 1, 2] } }, { list: { 1: true } });
await add("array/array-itself-hidden", list, { _sd: ["list"], list: { _sd: [0] } }, { list: { 0: true } });
await add(
  "array/elements-that-are-objects",
  { people: [{ n: "A", a: 1 }, { n: "B", a: 2 }] },
  { people: { _sd: [0, 1], 0: { _sd: ["a"] }, 1: { _sd: ["n"] } } },
  { people: { 0: { a: true }, 1: true } },
);
await add(
  "array/array-in-array",
  { m: [[1, 2], [3, 4]] },
  { m: { _sd: [1], 1: { _sd: [0] } } },
  { m: { 1: { 0: true } } },
);
await add("array/empty-and-null-elements", { a: [], b: [null, 0, ""] }, { b: { _sd: [0, 1, 2] } }, { b: { 0: true, 2: true } });

// ---- Decoys -----------------------------------------------------------------

await add("decoys/object-level", record, { _sd: ["name", "age"], _sd_decoy: 4 }, { name: true });
await add("decoys/array-level", list, { list: { _sd: [1], _sd_decoy: 3 } }, { list: { 1: true } });
await add("decoys/nothing-hidden-only-decoys", record, { _sd_decoy: 5 });
await add(
  "decoys/at-every-level",
  nested,
  { _sd: ["person"], _sd_decoy: 2, person: { _sd: ["name"], _sd_decoy: 2, address: { _sd: ["zip"], _sd_decoy: 2 } } },
  { person: { name: true, address: { zip: true } } },
);

// ---- Key Binding ------------------------------------------------------------

await add("key-binding/some-shown", record, { _sd: ["name", "age"] }, { name: true }, { keyBinding: true });
await add("key-binding/nothing-shown", record, { _sd: ["name", "age"] }, {}, { keyBinding: true });
await add("key-binding/array-elements", list, { list: { _sd: [0, 2] } }, { list: { 2: true } }, { keyBinding: true });

// ---- Seeded random trees ----------------------------------------------------

const pick = (items) => items[Math.floor(rng() * items.length)];
const between = (lo, hi) => lo + Math.floor(rng() * (hi - lo + 1));

function scalar() {
  return pick([
    () => `s${between(0, 999)}`,
    () => between(-50, 50_000),
    () => true,
    () => false,
    () => null,
    // Odd quarters: never whole, so the value is not renormalised to an integer.
    () => between(0, 400) * 0.5 + 0.25,
    () => "ünï☃ \"quoted\" \\ back",
    () => "",
  ])();
}

function value(depth) {
  const r = rng();
  if (depth >= 3 || r < 0.45) return scalar();
  return r < 0.75 ? object(depth + 1) : array(depth + 1);
}

const object = (depth) =>
  Object.fromEntries(Array.from({ length: between(1, 4) }, (_, i) => [`k${i}`, value(depth)]));
const array = (depth) => Array.from({ length: between(1, 4) }, () => value(depth));

// Array indices stay numbers, because a string index hides nothing (see `checkIssued`).
const entries = (v) => (Array.isArray(v) ? v.map((x, i) => [i, x]) : Object.entries(v));
const isContainer = (v) => v !== null && typeof v === "object";

function disclosureFrame(v) {
  const frame = {};
  const hidden = [];
  for (const [key, child] of entries(v)) {
    if (rng() < 0.5) hidden.push(key);
    if (isContainer(child)) {
      const inner = disclosureFrame(child);
      if (Object.keys(inner).length) frame[key] = inner;
    }
  }
  if (hidden.length) frame._sd = hidden;
  if (rng() < 0.2) frame._sd_decoy = between(1, 3);
  return frame;
}

function presentationFrame(v) {
  const frame = {};
  for (const [key, child] of entries(v)) {
    if (isContainer(child)) {
      const r = rng();
      if (r < 0.7) frame[key] = presentationFrame(child);
      else if (r < 0.85) frame[key] = true;
    } else if (rng() < 0.5) {
      frame[key] = true;
    }
  }
  return frame;
}

const TREES = 120;
for (let n = 0; n < TREES; n++) {
  const tree = object(0);
  const frame = disclosureFrame(tree);
  await add(`random/${n}/everything`, tree, frame);
  await add(`random/${n}/some`, tree, frame, presentationFrame(tree));
}

// ---- Forms this crate does not read -----------------------------------------

// RFC 9901 section 8 defines JSON serialisations. This crate reads the combined
// format only, so these must be refused, not misread.
{
  const issued = await sd.issue({ iss: "https://issuer.example", a: 1, b: 2 }, { _sd: ["a"] });
  cases.push({ id: "json/flattened", verdict: "refuse", token: JSON.stringify(sd.toFlattenJSON(issued)) });
  cases.push({ id: "json/general", verdict: "refuse", token: JSON.stringify(sd.toGeneralJSON(issued)) });
}

// ---- Credentials, for `dpp_vc` ----------------------------------------------

// What `dpp_vc::sd_jwt_vc::verify` checks beyond the mechanism: `iss` and `vct`
// present, `vct` as expected, `exp` and `nbf` against a given instant. The library
// has options for each, so it gives a verdict that is not this crate's own reading.
// It does not check `typ`, so nothing here rests on the profile's media type except
// that the library is told to set it, because `dpp_vc` requires it.
//
// RFC 9901 and RFC 7519 only. No conformance to the credential profile is claimed.
const VCT = "tag:example.test:vct:oracle:1";
const VC_NOW = 1_789_000_060;
const vcCases = [];

// `intended` is what the case was written to show. The library's verdict is what
// is recorded, but a case it judges the other way fails the run: otherwise a
// credential refused through a mistake here (a wrong `currentDate`, say) would be
// written down as a refusal, and this crate asked to agree with it.
//
// A bound credential names the Holder in `cnf`, and its presentation carries a Key
// Binding JWT made for `aud` (the Verifier's audience unless a case says
// otherwise). The library verifies it with Key Binding required, held to the
// challenge the case records, and so must `dpp_vc`.
const VC_AUDIENCE = "https://verifier.example";
async function addVc(id, claims, frame, { vct = VCT, presentation, intended = "accept", keyBinding, aud = VC_AUDIENCE } = {}) {
  try {
    const nonce = `nonce-${id}`;
    const issued = await sd.issue(keyBinding ? { ...claims, cnf: { jwk: holder.publicJwk } } : claims, frame, {
      header: { typ: "dc+sd-jwt" },
    });
    if (!checkIssued(id, frame, issued)) return;
    const token =
      presentation === undefined && !keyBinding
        ? issued
        : await sd.present(issued, presentation ?? {}, keyBinding ? { kb: { payload: { iat: VC_NOW, aud, nonce } } } : undefined);
    const challenge = keyBinding ? { nonce, aud: VC_AUDIENCE, now: VC_NOW, window: 0 } : undefined;
    const options = {
      currentDate: VC_NOW,
      expectedVct: vct,
      requiredClaimKeys: ["iss", "vct"],
      ...(challenge && { keyBindingNonce: nonce, expectedKeyBindingAudience: VC_AUDIENCE }),
    };
    let verdict = "accept";
    let expected;
    let why;
    try {
      expected = (await sd.verify(token, options)).payload;
    } catch (e) {
      verdict = "refuse";
      why = e?.message ?? e;
    }
    if (verdict !== intended) {
      problems.push(
        `${id}: meant to be ${intended === "accept" ? "accepted" : "refused"}, ` +
          `the library ${verdict === "accept" ? "accepts" : "refuses"} it${why ? ` (${why})` : ""}`,
      );
    }
    vcCases.push({ id, token, vct, verdict, ...(expected && { expected }), ...(challenge && { challenge }) });
  } catch (e) {
    problems.push(`${id}: the library cannot issue this credential: ${e?.message ?? e}`);
  }
}

const credential = { iss: "https://issuer.example", vct: VCT, iat: VC_NOW - 60, name: "Alice", tags: ["a", "b"], meta: { k: 1 } };

await addVc("vc/plain", credential, undefined);
await addVc("vc/concealed-all-shown", credential, { _sd: ["name", "meta"] });
await addVc("vc/concealed-none-shown", credential, { _sd: ["name", "meta"] }, { presentation: {} });
await addVc("vc/array-element-concealed", credential, { tags: { _sd: [1] } }, { presentation: { tags: { 1: true } } });
await addVc("vc/decoys", credential, { _sd: ["name"], _sd_decoy: 3 }, { presentation: { name: true } });
await addVc("vc/exp-in-the-future", { ...credential, exp: VC_NOW + 3600 }, undefined);
await addVc("vc/exp-already-passed", { ...credential, exp: VC_NOW - 1 }, undefined, { intended: "refuse" });
await addVc("vc/exp-is-this-very-second", { ...credential, exp: VC_NOW }, undefined, { intended: "refuse" });
await addVc("vc/nbf-already-reached", { ...credential, nbf: VC_NOW - 1 }, undefined);
await addVc("vc/nbf-not-yet", { ...credential, nbf: VC_NOW + 1 }, undefined, { intended: "refuse" });
await addVc("vc/fractional-exp", { ...credential, exp: VC_NOW + 0.5 }, undefined);
await addVc("vc/another-vct-than-expected", { ...credential, vct: "tag:example.test:vct:other:1" }, undefined, { intended: "refuse" });
await addVc("vc/no-vct", { iss: credential.iss, iat: credential.iat, name: "Alice" }, undefined, { intended: "refuse" });
await addVc("vc/no-iss", { vct: VCT, iat: credential.iat, name: "Alice" }, undefined, { intended: "refuse" });
await addVc("vc/bound-all-shown", credential, { _sd: ["name", "meta"] }, { keyBinding: true, presentation: { name: true, meta: true } });
await addVc("vc/bound-one-shown", credential, { _sd: ["name", "meta"] }, { keyBinding: true, presentation: { name: true } });
await addVc("vc/bound-nothing-shown", credential, { _sd: ["name", "meta"] }, { keyBinding: true, presentation: {} });
await addVc("vc/bound-array-element", credential, { tags: { _sd: ["1"] } }, { keyBinding: true, presentation: { tags: { 1: true } } });
await addVc("vc/bound-for-another-audience", credential, { _sd: ["name"] }, {
  keyBinding: true,
  presentation: { name: true },
  aud: "https://another-verifier.example",
  intended: "refuse",
});

// -----------------------------------------------------------------------------

if (problems.length) {
  console.error(`${problems.length} case(s) the library itself could not handle:`);
  for (const p of problems) console.error(`  ${p}`);
  process.exit(1);
}

// Calibration, not a check: how this issuer orders its own `_sd` arrays. It sorts them,
// as the RFC's examples do and as this crate does, which is why sortedness is a fair
// thing to ask of ours. Printed so that a change in the library shows up in the log.
// (The JSON forms are not compact tokens, and are left out.)
const compact = cases.filter((c) => c.verdict === "accept");
const sortedTokens = compact.filter((c) => sdArrays(c.token).every((a) => ascending(a))).length;

mkdirSync(path.dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify({ publicKey: issuer.publicJwk.x, cases }));
const vcPath = path.join(path.dirname(outPath), "issued-vc.json");
writeFileSync(vcPath, JSON.stringify({ publicKey: issuer.publicJwk.x, now: VC_NOW, cases: vcCases }));
console.log(
  `@sd-jwt/core ${libraryVersion} issued ${cases.length} tokens ` +
    `(${cases.filter((c) => c.keyBinding).length} with a Key Binding JWT, ` +
    `${cases.filter((c) => c.verdict === "refuse").length} this crate must refuse) to ${outPath}\n` +
    `and ${vcCases.length} credentials (${vcCases.filter((c) => c.verdict === "refuse").length} must be refused) to ${vcPath}\n` +
    `calibration: this issuer sorted every _sd array ascending in ${sortedTokens} of ${compact.length} tokens`,
);
