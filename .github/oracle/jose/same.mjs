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
 * presentation's own bytes differ between runs whatever its algorithm. Its payload text
 * is compared byte for byte, except that the signature of each such envelope is
 * replaced by its form.
 *
 * Nothing is parsed and written again before it is compared. Two payloads that differ
 * only in how a number is spelled sign different bytes, and must compare as different.
 *
 * A difference means the library changed what it writes, which is worth knowing
 * before it is a failure.
 */

import { readFileSync } from "node:fs";
import { VC_JWT_PREFIX, b64u, headerOf, isEcdsa, isHighS, signatureOf } from "./lib.mjs";

/** A `vc+jwt` enveloped in a payload's text: its prefix, the head `h.p.` and the signature. */
const ENVELOPE = /data:application\/vc\+jwt,([\w-]+\.[\w-]+\.)([\w-]*)/g;

/** The form of an ECDSA signature, the one thing about it that a fresh nonce leaves fixed. */
const formOf = (alg, token) => (isHighS(alg, signatureOf(token)) ? "high" : "low");

/** The decoded payload, as text. */
const payloadText = (token) => Buffer.from(token.split(".")[1], "base64url").toString();

/** Whether a token's payload encloses an ECDSA-signed `vc+jwt`, whose signature is then part of what the token signs. */
function encloses(token) {
  return [...payloadText(token).matchAll(ENVELOPE)].some(([whole]) =>
    isEcdsa(headerOf(whole.slice(VC_JWT_PREFIX.length)).alg),
  );
}

/** One enveloped token with its signature replaced by its form. Any other is kept as written. */
function reduceEnvelope(whole, head) {
  const token = whole.slice(VC_JWT_PREFIX.length);
  const alg = headerOf(token).alg;
  return isEcdsa(alg) ? `${VC_JWT_PREFIX}${head}${formOf(alg, token)}` : whole;
}

/**
 * The bytes a token's payload signs, for comparing: the payload segment as written,
 * unless it encloses an ECDSA-signed credential. Then the decoded text, with each such
 * signature replaced by its form and every other character kept as written.
 */
function payloadOf(token) {
  return encloses(token) ? payloadText(token).replace(ENVELOPE, reduceEnvelope) : token.split(".")[1];
}

/** What differs between two tokens, in what a fresh nonce cannot change. */
function differencesOf(a, b) {
  const found = [];
  const alg = headerOf(a).alg;
  if (a.split(".")[0] !== b.split(".")[0]) found.push("the protected headers differ");
  if (payloadOf(a) !== payloadOf(b)) found.push("the payloads differ");
  if (isEcdsa(alg)) {
    if (formOf(alg, a) !== formOf(alg, b)) found.push("the form of the signature differs");
  } else if (!encloses(a) && a.split(".")[2] !== b.split(".")[2]) {
    found.push("the signatures differ");
  }
  return found;
}

// ── controls, which run first ────────────────────────────────────────────────
// A comparison that cannot tell two payloads apart, or that splits two identical ones,
// proves nothing. The controls are built here, so they need no file.

const controlFailures = [];
const controlToken = (payload, fill) =>
  `${b64u(JSON.stringify({ alg: "ES256", kid: "control" }))}.${b64u(payload)}.${b64u(Buffer.alloc(64, fill))}`;

const spelledAsZero = controlToken('{"zero":0}', 1);
const spelledAsZeroPoint = controlToken('{"zero":0.0}', 2);
const sameText = controlToken('{"zero":0}', 3);

const negative = differencesOf(spelledAsZero, spelledAsZeroPoint);
if (!negative.includes("the payloads differ")) {
  controlFailures.push(`negative control: payloads that differ only in spelling compared as ${negative.length ? negative : "equal"}`);
}
const positive = differencesOf(spelledAsZero, sameText);
if (positive.length > 0) controlFailures.push(`positive control: identical payload text compared as different: ${positive}`);

if (controlFailures.length > 0) {
  for (const f of controlFailures) console.log(`  ${f}`);
  process.exit(1);
}

// ── the two files ────────────────────────────────────────────────────────────

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
  return `${header}.${payload}|${formOf(alg, token)}`;
}

/** A document with each enveloped `vc+jwt` reduced. */
function reducedDocument(document) {
  return JSON.stringify(document, (_key, value) =>
    typeof value === "string" && value.startsWith(VC_JWT_PREFIX)
      ? VC_JWT_PREFIX + reduced(value.slice(VC_JWT_PREFIX.length))
      : value,
  );
}

function compare(id, a, b) {
  tokens += 1;
  if (!isEcdsa(headerOf(a).alg) && !encloses(a)) exact += 1;
  for (const d of differencesOf(a, b)) differences.push(`${id}: ${d}`);
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
