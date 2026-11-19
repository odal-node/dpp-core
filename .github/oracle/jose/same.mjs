/**
 * Whether two runs of `issue.mjs` issued the same thing.
 *
 * Ed25519 is deterministic, so its tokens must be byte for byte the same. ECDSA
 * through Web Crypto draws a fresh nonce for every signature, so its signatures
 * differ on every run and cannot be compared. What is compared for them is what the
 * signature covers, the protected header and the payload, and which form each
 * signature has, low-S or high-S, since `issue.mjs` fixes that on purpose.
 *
 * A presentation's payload holds the compact JWS of each credential it envelops. When
 * one of those is ECDSA its signature is part of the presentation's payload, so the
 * presentation's own bytes differ between runs whatever its algorithm. Its payload is
 * compared with each such envelope reduced to its header, its payload and the form of
 * its signature.
 *
 * A difference means the library changed what it writes, which is worth knowing
 * before it is a failure.
 */

import { readFileSync } from "node:fs";
import { VC_JWT_PREFIX, headerOf, isEcdsa, isHighS, signatureOf } from "./lib.mjs";

const [firstPath, secondPath] = process.argv.slice(2);
const read = (p) => JSON.parse(readFileSync(p, "utf8"));
const [first, second] = [read(firstPath), read(secondPath)];

const differences = [];
let tokens = 0;
let exact = 0;

/** A token with what a fresh nonce can change taken out of it. */
function reduced(token) {
  const alg = headerOf(token).alg;
  const [header, payload] = token.split(".");
  if (!isEcdsa(alg)) return `${header}.${payload}`;
  return `${header}.${payload}|${isHighS(alg, signatureOf(token)) ? "high" : "low"}`;
}

/** A document with each enveloped `vc+jwt` reduced. */
function reducedDocument(document) {
  return JSON.stringify(document, (_key, value) =>
    typeof value === "string" && value.startsWith(VC_JWT_PREFIX)
      ? VC_JWT_PREFIX + reduced(value.slice(VC_JWT_PREFIX.length))
      : value,
  );
}

/** The outer token's payload with each envelope reduced, as text. */
function reducedPayload(token) {
  try {
    return reducedDocument(JSON.parse(Buffer.from(token.split(".")[1], "base64url")));
  } catch {
    return "unreadable";
  }
}

function compare(id, a, b) {
  tokens += 1;
  const alg = headerOf(a).alg;
  const envelopes = (t) => JSON.stringify(Buffer.from(t.split(".")[1], "base64url").toString()).includes("vc+jwt,");
  if (!isEcdsa(alg) && !envelopes(a)) {
    exact += 1;
    if (a !== b) differences.push(`${id}: the ${alg} tokens differ`);
    return;
  }
  if (a.split(".")[0] !== b.split(".")[0]) differences.push(`${id}: the protected headers differ`);
  if (reducedPayload(a) !== reducedPayload(b)) differences.push(`${id}: the payloads differ`);
  if (isEcdsa(alg) && isHighS(alg, signatureOf(a)) !== isHighS(alg, signatureOf(b))) {
    differences.push(`${id}: the form of the signature differs`);
  }
}

if (first.cases.length !== second.cases.length) differences.push("the number of cases differs");
if (JSON.stringify(first.keys) !== JSON.stringify(second.keys)) differences.push("the keys differ");
for (const [i, a] of first.cases.entries()) {
  const b = second.cases[i];
  if (!b || a.id !== b.id || reducedDocument(a.document) !== reducedDocument(b.document)) {
    differences.push(`${a.id}: the case differs`);
    continue;
  }
  compare(a.id, a.token, b.token);
  for (const [j, e] of (a.envelopes ?? []).entries()) compare(`${a.id}[${j}]`, e.token, b.envelopes[j].token);
}

console.log(`${tokens} tokens compared; ${exact} byte for byte, ${tokens - exact} by what they sign and the form of the signature`);
if (differences.length > 0) {
  for (const d of differences) console.log(`  ${d}`);
  process.exit(1);
}
