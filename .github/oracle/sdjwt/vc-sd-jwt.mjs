/**
 * Judge the `vc+sd-jwt` and `vp+sd-jwt` tokens `dpp_vc::jose` issues, with an
 * independent implementation of RFC 9901.
 *
 * `vc-sd-jwt.json` comes from
 * `EMIT_VC_SD_JWT_CORPUS=1 cargo test -p dpp-vc --lib jose::sd_jwt_oracle_tests`.
 * Each case is a W3C Verifiable Credential or Presentation secured as an SD-JWT
 * (VC-JOSE-COSE section 3.2), as issued or as presented in part. The cases include
 * the three issuance inputs of the W3C VC-JOSE-COSE test suite (rows 17 to 19),
 * issued under the suite's own P-384 and P-521 keys.
 *
 *   signature and processing  The library verifies every token with the published
 *                             key and reads the claims. They must be the claims this
 *                             crate reads, and the claims the document gives once
 *                             the concealed claims that were not presented are
 *                             taken out. A credential issued whole must read as the
 *                             document it was made from.
 *   the Disclosures           Every Disclosure's digest, recomputed here, is one the
 *                             signed payload refers to, in an `_sd` array or as the
 *                             `...` placeholder of an array element, in the payload
 *                             itself or inside another Disclosure. A token issued
 *                             whole refers to no digest it has no Disclosure for,
 *                             since this crate issues no decoys.
 *   `_sd_alg`                 `sha-256`, and present exactly when something is hidden.
 *   the header                `typ` is `vc+sd-jwt` or `vp+sd-jwt`, `cty` is `vc` or
 *                             `vp`, `alg` is the key's, and `kid` is the one the
 *                             case names.
 *   hidden claims             A concealed string does not appear in the payload as
 *                             signed.
 *   order                     Every `_sd` array is in ascending order (clause 4.2.4.1).
 *   Key Binding               A case with a `challenge` is a presentation bound to the
 *                             Holder's key, which the signer wrote as `cnf`, and is judged
 *                             with Key Binding required at that challenge: the library
 *                             takes the Holder key from `cnf` and checks the Key Binding
 *                             JWT's signature, `typ`, `sd_hash`, nonce, audience and `iat`
 *                             window. The case's `accepts` is the verdict every reader
 *                             must reach. The controls are accepted; the negative controls
 *                             (a wrong nonce, a wrong audience, an `sd_hash` that belongs
 *                             to another presentation, and no Key Binding JWT at all)
 *                             must be refused by the library and by this crate.
 *
 * **RFC 9901 level, and the VC-JOSE-COSE media types.** The library knows nothing
 * of `vc+sd-jwt`: what is judged against section 3.2 is checked here, from the
 * header. As in `judge.mjs`, the signature is checked by Node's `crypto` in
 * `lib.mjs`, not by the library, and nothing here is evidence of conformance.
 *
 * Reports every failure and exits non-zero if there is one.
 */

import { createHash, generateKeyPairSync } from "node:crypto";
import { readFileSync } from "node:fs";
import {
  ascending,
  judge,
  keyBindingOptions,
  libraryVersion,
  same,
  sdArrays,
  verifyingInstance,
} from "./lib.mjs";

const corpusPath = process.argv[2] ?? "target/sdjwt-oracle/vc-sd-jwt.json";
const corpus = JSON.parse(readFileSync(corpusPath, "utf8"));

const failures = [];
const fail = (id, why) => failures.push(`${id}: ${why}`);

const decode = (segment) => JSON.parse(Buffer.from(segment, "base64url"));
const split = (token) => {
  const [jwt, ...rest] = token.split("~");
  return { jwt, last: rest.pop(), disclosures: rest };
};
const sha256 = (text) => createHash("sha256").update(text).digest("base64url");

/** Every digest a value refers to: `_sd` arrays, and the `...` placeholder of an array element. */
function referencedDigests(value, found = []) {
  if (Array.isArray(value)) {
    value.forEach((item) => referencedDigests(item, found));
  } else if (value && typeof value === "object") {
    const keys = Object.keys(value);
    if (keys.length === 1 && keys[0] === "..." && typeof value["..."] === "string") found.push(value["..."]);
    for (const [key, inner] of Object.entries(value)) {
      if (key === "_sd" && Array.isArray(inner)) found.push(...inner.filter((d) => typeof d === "string"));
      referencedDigests(inner, found);
    }
  }
  return found;
}

/** The value of `document` at a path of names and positions, or undefined. */
function at(document, path) {
  return path.reduce((node, step) => (node === undefined || node === null ? undefined : node[step]), document);
}

console.log(`independent implementation: @sd-jwt/core ${libraryVersion}`);
console.log(`node ${process.version}, ${corpus.cases.length} tokens in ${corpusPath}`);

// ---- Negative controls ------------------------------------------------------

const instanceFor = (c) => verifyingInstance(c.jwk, c.alg);
{
  const problems = [];
  const issued = corpus.cases.find((c) => c.id === "suite/row-17/issued");
  if (!issued || !(await judge(instanceFor(issued), issued.token)).accepts) {
    problems.push("an issued credential is refused, so nothing it refuses means anything");
  } else {
    const { jwt, disclosures } = split(issued.token);
    const [h, p, s] = jwt.split(".");
    const flipped = [[h, p, (s[0] === "A" ? "B" : "A") + s.slice(1)].join("."), ...disclosures, ""].join("~");
    if ((await judge(instanceFor(issued), flipped)).accepts) problems.push("a changed signature was accepted");
    const stranger = generateKeyPairSync("ec", { namedCurve: "P-384" }).publicKey.export({ format: "jwk" });
    if ((await judge(verifyingInstance(stranger, issued.alg), issued.token)).accepts) {
      problems.push("a stranger's key was accepted");
    }
    // A Disclosure no digest refers to: clause 7.1 step 5 rejects it.
    const extra = Buffer.from(JSON.stringify(["c2FsdA", "extra", "x"])).toString("base64url");
    if ((await judge(instanceFor(issued), `${issued.token}${extra}~`)).accepts) {
      problems.push("a Disclosure that matches no digest was accepted");
    }
  }
  // Key Binding has to be able to fail too: the bound control, another nonce.
  const boundControl = corpus.cases.find((c) => c.id === "bound/vc-ed25519/valid");
  if (!boundControl) {
    problems.push("the corpus has no bound control");
  } else {
    const options = keyBindingOptions(boundControl.challenge);
    if (!(await judge(instanceFor(boundControl), boundControl.token, options)).accepts) {
      problems.push("a bound control is refused, so nothing it refuses with Key Binding means anything");
    }
    const replayed = { ...options, keyBindingNonce: "another nonce" };
    if ((await judge(instanceFor(boundControl), boundControl.token, replayed)).accepts) {
      problems.push("a bound control was accepted at another nonce");
    }
  }
  if (ascending(["b", "a"]) || !ascending(["a", "b"])) problems.push("the ascending check is wrong on a plain list");
  if (problems.length) {
    console.error("NEGATIVE CONTROLS FAILED; the judge cannot be trusted:");
    for (const p of problems) console.error(`  ${p}`);
    process.exit(2);
  }
  console.log("negative controls: the judge refuses what it must");
}

// ---- The tokens -------------------------------------------------------------

const algs = new Set();
let withDisclosures = 0;
let orderArrays = 0;
let boundCases = 0;
let refusedAsRequired = 0;
for (const c of corpus.cases) {
  algs.add(c.alg);
  const { jwt, last, disclosures } = split(c.token);
  // A Key Binding JWT is the Holder's, so only a case bound to a Holder carries one.
  const bound = c.challenge !== undefined;
  if (!bound && last !== "") fail(c.id, "the token carries a Key Binding JWT, and this crate issues none");
  if (bound) boundCases++;

  // The header: section 3.2 says SHOULD for `typ` and `cty`, and this crate writes both.
  const header = decode(jwt.split(".")[0]);
  const expectedType = c.kind === "credential" ? ["vc+sd-jwt", "vc"] : ["vp+sd-jwt", "vp"];
  if (header.typ !== expectedType[0]) fail(c.id, `typ is ${header.typ}, expected ${expectedType[0]}`);
  if (header.cty !== expectedType[1]) fail(c.id, `cty is ${header.cty}, expected ${expectedType[1]}`);
  if (header.typ !== c.typ || header.cty !== c.cty) fail(c.id, "the header disagrees with the corpus");
  if (header.alg !== c.alg) fail(c.id, `alg is ${header.alg}, the key signs under ${c.alg}`);
  if (header.kid !== c.kid) fail(c.id, `kid is ${header.kid}, expected ${c.kid}`);

  // The Disclosures and their digests, recomputed here.
  const signed = decode(jwt.split(".")[1]);
  const referenced = new Set(referencedDigests(signed));
  for (const encoded of disclosures) {
    for (const digest of referencedDigests(decode(encoded).at(-1))) referenced.add(digest);
  }
  const held = new Set(disclosures.map(sha256));
  for (const digest of held) {
    if (!referenced.has(digest)) fail(c.id, `a Disclosure's digest ${digest} is referred to nowhere`);
  }
  if (c.id.endsWith("/issued") && referenced.size !== held.size) {
    fail(c.id, `${referenced.size} digests are referred to and ${held.size} Disclosures travel, and no decoys are issued`);
  }
  if (held.size !== disclosures.length) fail(c.id, "two Disclosures have one digest");
  if (disclosures.length > 0) withDisclosures++;
  if (referenced.size > 0 !== (signed._sd_alg === "sha-256")) {
    fail(c.id, `_sd_alg is ${signed._sd_alg}, and ${referenced.size} digests are referred to`);
  }
  for (const digests of sdArrays(c.token)) {
    if (digests.length < 2) continue;
    orderArrays++;
    if (!ascending(digests)) fail(c.id, "an _sd array is not in ascending order");
  }

  // What was concealed is not in the signed payload.
  const signedText = JSON.stringify(signed);
  for (const path of c.concealed) {
    const value = at(c.document, path);
    if (typeof value === "string" && value.length > 3 && signedText.includes(JSON.stringify(value))) {
      fail(c.id, `the concealed claim at ${JSON.stringify(path)} is in the signed payload`);
    }
  }

  // The library, with Key Binding required when the case names a challenge.
  const theirs = await judge(instanceFor(c), c.token, bound ? keyBindingOptions(c.challenge) : {});
  if (bound && c.accepts === false) {
    // A negative control: every reader must refuse it, for the rule it breaks.
    if (theirs.accepts) fail(c.id, "the library accepts a token that must be refused");
    if (c.ours.accepts) fail(c.id, "this crate accepts a token that must be refused");
    refusedAsRequired++;
    continue;
  }
  if (!theirs.accepts) {
    fail(c.id, `the library refuses it: ${theirs.error}`);
    continue;
  }
  if (!c.ours.accepts) fail(c.id, `this crate refuses its own token: ${c.ours.error}`);
  else if (!same(c.ours.payload, theirs.payload)) fail(c.id, "this crate and the library read different claims");
  if (!same(theirs.payload, c.expected)) fail(c.id, "the library does not read the document this was issued from");
}

for (const id of ["suite/row-17/issued", "suite/row-18/issued", "suite/row-19/issued"]) {
  if (!corpus.cases.some((c) => c.id === id)) fail(id, "the corpus has no such case");
}

console.log(`${corpus.cases.length} tokens, ${withDisclosures} with Disclosures; algorithms ${[...algs].sort().join(", ")}`);
console.log(`Key Binding required: ${boundCases} bound cases, ${refusedAsRequired} of them negative controls refused by the library and by this crate`);
console.log(`claim order (clause 4.2.4.1): ${orderArrays} _sd arrays of two or more digests; each strictly ascending`);

if (failures.length) {
  console.error(`\n${failures.length} FAILURE(S):`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}
console.log("every token agrees");
