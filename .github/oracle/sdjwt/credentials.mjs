/**
 * Judge the SD-JWT VCs this workspace issues, one set per product group, with an
 * independent implementation of RFC 9901.
 *
 * `credentials.json` comes from
 * `EMIT_SDJWT_VC_CORPUS=1 cargo test -p dpp-tests --test sdjwt_vc_oracle_corpus`.
 * Each case is a credential, or a presentation of one, built by
 * `dpp_vc::sd_jwt_vc::issue` from a product group's schema fixture and the
 * disclosure classes of its current schema version.
 *
 *   signature and processing  The library verifies every token, with the issuer's
 *                             public key, and reads the claims. They must be the
 *                             claims this crate reads.
 *   the passport              The credential as issued must reveal exactly the
 *                             passport that went in, plus the `iss`, `vct` and
 *                             `iat` the issuer sets. Concealment that lost or
 *                             invented a claim shows here.
 *   the `kid`                 Recomputed from the public key as the RFC 7638
 *                             thumbprint in the RFC 9278 URI form, and compared
 *                             with the `kid` in every JWT header.
 *
 * **RFC 9901 level only.** The credential profile (the `dc+sd-jwt` type, `vct`) is
 * an Internet-Draft and nothing here is evidence of conformance to it; the `typ`
 * is checked only so that a change to it is noticed. As in `judge.mjs`, the
 * signature is checked by Node's `crypto` in `lib.mjs`, not by the library.
 *
 * Reports every failure and exits non-zero if there is one.
 */

import { createHash, generateKeyPairSync } from "node:crypto";
import { readFileSync } from "node:fs";
import { ascending, judge, libraryVersion, same, sdArrays, verifyingInstance } from "./lib.mjs";

const corpusPath = process.argv[2] ?? "target/sdjwt-oracle/credentials.json";
const corpus = JSON.parse(readFileSync(corpusPath, "utf8"));

const failures = [];
const fail = (id, why) => failures.push(`${id}: ${why}`);

/** RFC 7638 for an OKP key, then the URI form of RFC 9278. */
function thumbprintUri(jwk) {
  // The required members, in lexicographic order, with no whitespace.
  const canonical = `{"crv":${JSON.stringify(jwk.crv)},"kty":${JSON.stringify(jwk.kty)},"x":${JSON.stringify(jwk.x)}}`;
  const digest = createHash("sha256").update(canonical).digest("base64url");
  return `urn:ietf:params:oauth:jwk-thumbprint:sha-256:${digest}`;
}

const headerOf = (token) => JSON.parse(Buffer.from(token.split(".")[0], "base64url"));

console.log(`independent implementation: @sd-jwt/core ${libraryVersion}`);
console.log(`node ${process.version}, ${corpus.cases.length} credentials in ${corpusPath}`);

// ---- Negative controls ------------------------------------------------------

const instance = verifyingInstance(corpus.issuerJwk, "EdDSA");
const options = { currentDate: corpus.currentDate };
const issued = corpus.cases.find((c) => c.id.endsWith("/issued"));

{
  const problems = [];
  if (!issued || !(await judge(instance, issued.token, options)).accepts) {
    problems.push("an issued credential is refused, so nothing it refuses means anything");
  } else {
    const [jwt, ...rest] = issued.token.split("~");
    const [h, p, s] = jwt.split(".");
    const flipped = [[h, p, (s[0] === "A" ? "B" : "A") + s.slice(1)].join("."), ...rest].join("~");
    if ((await judge(instance, flipped, options)).accepts) problems.push("a changed signature was accepted");
  }
  const stranger = generateKeyPairSync("ed25519").publicKey.export({ format: "jwk" });
  if (thumbprintUri(stranger) === corpus.kid) problems.push("two different keys have one thumbprint");
  if (ascending(["b", "a"]) || !ascending(["a", "b"])) problems.push("the ascending check is wrong on a plain list");
  // RFC 7638 section 3.1's own example is an RSA key; the OKP member order is
  // fixed by the same rule (crv, kty, x), checked here against RFC 8037 Appendix A.3.
  const rfc8037 = { kty: "OKP", crv: "Ed25519", x: "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo" };
  if (thumbprintUri(rfc8037) !== "urn:ietf:params:oauth:jwk-thumbprint:sha-256:kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k") {
    problems.push("the thumbprint of the RFC 8037 Appendix A.3 key is not what that RFC prints");
  }
  if (problems.length) {
    console.error("NEGATIVE CONTROLS FAILED; the judge cannot be trusted:");
    for (const p of problems) console.error(`  ${p}`);
    process.exit(2);
  }
  console.log("negative controls: the judge refuses what it must");
}

// ---- The credentials --------------------------------------------------------

const expectedKid = thumbprintUri(corpus.issuerJwk);
if (corpus.kid !== expectedKid) fail("corpus", `the corpus says the kid is ${corpus.kid}, the key's thumbprint is ${expectedKid}`);

const groups = new Set();
let orderArrays = 0;
for (const c of corpus.cases) {
  groups.add(c.productGroup);
  // Clause 4.2.4.1: the order of an _sd array must not be the order of the claims. Sorted
  // is a function of the salted digests alone, and is the method the clause names.
  for (const digests of sdArrays(c.token)) {
    if (digests.length < 2) continue;
    orderArrays++;
    if (!ascending(digests)) fail(c.id, "an _sd array is not in ascending order");
  }
  const header = headerOf(c.token);
  if (header.kid !== expectedKid) fail(c.id, `kid ${header.kid} is not the thumbprint of the signing key`);
  if (header.alg !== "EdDSA") fail(c.id, `alg is ${header.alg}`);
  if (header.typ !== "dc+sd-jwt") fail(c.id, `typ is ${header.typ}`);

  const theirs = await judge(instance, c.token, options);
  if (!c.ours.accepts) fail(c.id, `this crate refuses its own credential: ${c.ours.error}`);
  if (!theirs.accepts) {
    fail(c.id, `the library refuses it: ${theirs.error}`);
    continue;
  }
  if (c.ours.accepts && !same(c.ours.payload, theirs.payload)) {
    fail(c.id, "this crate and the library read different claims");
  }
  if (c.expected && !same(theirs.payload, c.expected)) {
    fail(c.id, "the library does not read the passport this was issued from");
  }
}

console.log(`${corpus.cases.length} credentials across ${groups.size} product groups; kid ${expectedKid}`);
console.log(`claim order (clause 4.2.4.1): ${orderArrays} _sd arrays of two or more digests; each strictly ascending`);

if (failures.length) {
  console.error(`\n${failures.length} FAILURE(S):`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}
console.log("every credential agrees");
