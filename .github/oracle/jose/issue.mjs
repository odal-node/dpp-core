/**
 * The other direction: credentials and presentations an independent JOSE library
 * signs, for `dpp_vc::jose` to verify.
 *
 * `judge.mjs` asks whether the library reads what this crate signed. This script
 * has the library sign, under EdDSA, ES256, ES384 and ES512 and the suite's own
 * keys: six credentials under each algorithm, and two presentations under each, each
 * enveloping one credential. `dpp_vc::jose::verify_credential`, `verify_presentation`
 * and `verify_credentials` read them (`jose::oracle_tests::reverse`, run with
 * `CHECK_JOSE_ORACLE_ISSUED`).
 *
 * Locked rule: this workspace signs low-S and verifies both. Node's ECDSA does not
 * normalise its signatures, so some of what it signs has the high `S`, and the Rust
 * check holds the file to having both. The signing is repeated until each signature
 * has the form this script wants (see `sign` in `lib.mjs`), alternating, so that the
 * corpus holds both on purpose and the file is the same shape on every run.
 *
 * The documents differ from this crate's in what is not meaning: the payload is
 * `JSON.stringify` of the document, not RFC 8785 canonical form, and one credential
 * carries a member that is in no vocabulary, which a verifier ignores (VC-JOSE-COSE
 * section 3.1.3).
 *
 * The library verifies each token it issued before it is written, with the same
 * checks `judge.mjs` makes, so a case the library cannot read itself is reported
 * here and fails the run, and is not just missing.
 *
 * Writes `target/jose-oracle/issued.json`:
 *   { library, keys: { <kid>: <public JWK> }, cases: [{ id, kind, alg, kid, token,
 *     document, now?, envelopes?: [{ index, kid, token, document }] }] }
 */

import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  ALGORITHMS,
  isEcdsa,
  isHighS,
  jose,
  libraryVersion,
  sign,
  signatureOf,
  suiteKeys,
} from "./lib.mjs";

const outPath = process.argv[2] ?? "target/jose-oracle/issued.json";
const { keys, published } = suiteKeys();

const base = () => ({
  "@context": ["https://www.w3.org/ns/credentials/v2"],
  id: "urn:uuid:7b0a4c1e-3f5d-4c8e-9a41-2d6f8e0b1c33",
  type: ["VerifiableCredential"],
  issuer: "https://issuer.example",
  validFrom: "2025-01-01T00:00:00Z",
  credentialSubject: { id: "did:example:subject", name: "Example" },
});

/**
 * Six credentials. `useThumbprint` names the key by the thumbprint of its JWK and not
 * by the URL of its verification method, which VC-JOSE-COSE section 4.2 allows when
 * the credential has an `iss`.
 */
const shapes = () => {
  const claims = { ...base(), iss: "https://issuer.example", nbf: 1_700_000_000, exp: 4_000_000_000, iat: 1_700_000_000 };
  const text = base();
  text.issuer = { id: "https://issuer.example", name: "Zażółć gęślą jaźń" };
  text.credentialSubject = [
    { id: "did:example:one", name: "日本語 ✓ 😀", nested: { a: [1, { b: null }], c: "\u007f\"\\" } },
    { id: "did:example:two", name: "Example" },
  ];
  const numbers = base();
  numbers.credentialSubject = { id: "did:example:subject", small: 0.000001, large: 1e21, negative: -0.5, integer: 9007199254740991, zero: 0 };
  const object = base();
  object.issuer = { id: "https://issuer.example" };
  object.validUntil = "2099-01-01T00:00:00Z";
  const unknown = { ...base(), notInAnyVocabulary: { a: 1 } };
  return [
    { shape: "base", document: base() },
    { shape: "jwt-claims", document: claims, now: 1_800_000_000, useThumbprint: true },
    { shape: "text", document: text },
    { shape: "numbers", document: numbers },
    { shape: "issuer-object", document: object },
    { shape: "undeclared-member", document: unknown },
  ];
};

const cases = [];
const problems = [];
/** How many ECDSA signatures each algorithm has made, to alternate high and low. */
const made = Object.fromEntries(Object.keys(ALGORITHMS).map((a) => [a, 0]));
const wantHigh = (alg) => (isEcdsa(alg) ? made[alg]++ % 2 === 0 : undefined);

/** Verifies what the library issued, as the library, with the algorithm pinned. */
async function selfCheck(id, token, alg) {
  try {
    await jose.compactVerify(token, await jose.importJWK(keys[alg].publicJwk, alg), { algorithms: [alg] });
  } catch (e) {
    problems.push(`${id}: the library cannot read its own token: ${e?.message ?? e}`);
  }
}

async function issue(id, alg, kid, typ, cty, document) {
  const token = await sign(keys[alg].secretJwk, alg, { alg, kid, typ, cty }, document, wantHigh(alg));
  await selfCheck(id, token, alg);
  return token;
}

for (const alg of Object.keys(ALGORITHMS)) {
  for (const { shape, document, now, useThumbprint } of shapes()) {
    const kid = useThumbprint ? keys[alg].thumbprint : keys[alg].id;
    const id = `credential/${alg}/${shape}`;
    const token = await issue(id, alg, kid, "vc+jwt", "vc", document);
    cases.push({ id, kind: "vc", alg, kid, token, document, ...(now ? { now } : {}) });
  }
  // Each presentation envelopes a credential signed under one algorithm, rotating,
  // so every algorithm's credentials appear in a presentation under every other.
  for (const n of [0, 1]) {
    const issuerAlg = Object.keys(ALGORITHMS)[(Object.keys(ALGORITHMS).indexOf(alg) + n) % 4];
    const inner = base();
    inner.id = `urn:uuid:5a4c1b7e-0000-4000-8000-00000000000${n}`;
    const innerKid = keys[issuerAlg].id;
    const innerId = `presentation/${alg}/${n}/envelope`;
    const innerToken = await issue(innerId, issuerAlg, innerKid, "vc+jwt", "vc", inner);
    const document = {
      "@context": ["https://www.w3.org/ns/credentials/v2"],
      id: `urn:uuid:0d9c6b0e-5a3c-4f43-8f0e-6f2a8d1c9e5${n}`,
      type: ["VerifiablePresentation"],
      holder: "https://holder.example",
      verifiableCredential: [
        {
          "@context": "https://www.w3.org/ns/credentials/v2",
          type: "EnvelopedVerifiableCredential",
          id: `data:application/vc+jwt,${innerToken}`,
        },
      ],
    };
    const id = `presentation/${alg}/${n}`;
    const token = await issue(id, alg, keys[alg].id, "vp+jwt", "vp", document);
    cases.push({
      id, kind: "vp", alg, kid: keys[alg].id, token, document,
      envelopes: [{ index: 0, kid: innerKid, token: innerToken, document: inner }],
    });
  }
}

// What the file holds, counted from the signatures themselves.
const form = {};
const tally = (token) => {
  const alg = JSON.parse(Buffer.from(token.split(".")[0], "base64url")).alg;
  if (!isEcdsa(alg)) return;
  form[alg] ??= { high: 0, low: 0 };
  form[alg][isHighS(alg, signatureOf(token)) ? "high" : "low"] += 1;
};
for (const c of cases) {
  tally(c.token);
  for (const e of c.envelopes ?? []) tally(e.token);
}
for (const alg of Object.keys(ALGORITHMS).filter(isEcdsa)) {
  if (!form[alg]?.high || !form[alg]?.low) problems.push(`${alg}: the file lacks a high-S or a low-S signature: ${JSON.stringify(form[alg])}`);
}

mkdirSync(path.dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify({ library: { name: "jose", version: libraryVersion }, keys: published, cases }));

console.log(`jose ${libraryVersion}`);
console.log(`issued ${cases.length} cases (${cases.filter((c) => c.kind === "vc").length} credentials, ${cases.filter((c) => c.kind === "vp").length} presentations) to ${outPath}`);
for (const [alg, f] of Object.entries(form)) console.log(`${alg}: ${f.high} high-S, ${f.low} low-S signatures`);
if (problems.length > 0) {
  console.log(`\n${problems.length} problem(s):`);
  for (const p of problems) console.log(`  ${p}`);
  process.exit(1);
}
