/**
 * Shared by `judge.mjs`, `issue.mjs` and `same.mjs`: the independent JOSE library,
 * the suite's keys, and what the order of each curve says about an ECDSA `S`.
 *
 * The library is `jose` (panva). It does the JWS processing and the signature
 * check, through the Web Crypto API of Node, which is OpenSSL: it shares no code
 * with this crate's `ring`-free RustCrypto stack. Say that plainly wherever this
 * oracle is cited.
 *
 * It is looked up in `JOSE_ORACLE_MODULES`, or else next to this file (where CI
 * installs it). A dynamic import of a file URL, because ESM ignores `NODE_PATH`
 * and the pinned install may sit outside the repository.
 */

import { readFileSync } from "node:fs";
import path from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
export const modulesDir = process.env.JOSE_ORACLE_MODULES ?? path.join(here, "node_modules");

export const jose = await import(
  pathToFileURL(path.join(modulesDir, "jose", "dist", "webapi", "index.js")).href
);
const { CompactSign } = jose;
export const libraryVersion = JSON.parse(
  readFileSync(path.join(modulesDir, "jose", "package.json"), "utf8"),
).version;

/** The suite's key files, which sign everything in both directions. */
const suiteDir = path.join(here, "../../../crates/dpp-vc/src/jose/fixtures/w3c-vc-jose-cose");

/** Each algorithm this crate signs under, with the suite file of its key. */
export const ALGORITHMS = {
  EdDSA: "vm-ed25519.json",
  ES256: "vm-p256.json",
  ES384: "vm-p384.json",
  ES512: "vm-p521.json",
};

/** The algorithm a JWK is for, from its `kty` and `crv` alone. */
export function algorithmOf(jwk) {
  if (jwk.kty === "OKP" && jwk.crv === "Ed25519") return "EdDSA";
  if (jwk.kty === "EC") return { "P-256": "ES256", "P-384": "ES384", "P-521": "ES512" }[jwk.crv];
  return undefined;
}

/**
 * The suite's keys: `{ [alg]: { id, thumbprint, publicJwk, secretJwk } }`, and the
 * `kid` map a verifier resolves a token's `kid` in. A token of the suite's names a
 * key by the thumbprint in its JWK's `kid`, and a token of this crate's by the
 * `id` of its verification method, so each key is listed under both.
 */
export function suiteKeys() {
  const keys = {};
  const published = {};
  for (const [alg, file] of Object.entries(ALGORITHMS)) {
    const body = JSON.parse(readFileSync(path.join(suiteDir, file), "utf8"));
    keys[alg] = {
      id: body.id,
      thumbprint: body.publicKeyJwk.kid,
      publicJwk: body.publicKeyJwk,
      secretJwk: body.secretKeyJwk,
    };
    published[body.id] = body.publicKeyJwk;
    published[body.publicKeyJwk.kid] = body.publicKeyJwk;
  }
  return { keys, published };
}

/** The order `n` of each curve, FIPS 186-5 appendix D.1.2 and SEC 2. */
const ORDERS = {
  ES256: 0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551n,
  ES384:
    0xffffffffffffffffffffffffffffffffffffffffffffffffc7634d81f4372ddf581a0db248b0a77aecec196accc52973n,
  ES512:
    0x01fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa51868783bf2f966b7fcc0148f709a5d03bb5c9b8899c47aebb6fb71e91386409n,
};

export const isEcdsa = (alg) => alg in ORDERS;

const toBig = (bytes) => BigInt("0x" + (Buffer.from(bytes).toString("hex") || "0"));

/** The `S` of an `R || S` signature. */
function sOf(alg, signature) {
  const width = signature.length / 2;
  return { width, s: toBig(signature.subarray(width)) };
}

/** Whether the signature's `S` is above `n / 2`: the form this crate never writes. */
export function isHighS(alg, signature) {
  const { s } = sOf(alg, signature);
  return s > ORDERS[alg] / 2n;
}

/** `R || (n - S)`: the other valid signature over the same message. */
export function twin(alg, signature) {
  const { width, s } = sOf(alg, signature);
  const hex = (ORDERS[alg] - s).toString(16).padStart(2 * width, "0");
  return Buffer.concat([signature.subarray(0, width), Buffer.from(hex, "hex")]);
}

export const b64u = (bytes) => Buffer.from(bytes).toString("base64url");

export const signatureOf = (token) => Buffer.from(token.split(".")[2], "base64url");

export const headerOf = (token) => JSON.parse(Buffer.from(token.split(".")[0], "base64url"));

/** Key order is not meaning in JSON, so compare with it removed. */
export function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((k) => [k, canonical(value[k])]),
    );
  }
  return value;
}

export const same = (a, b) => JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));

/** The media-type prefix of an Enveloped Verifiable Credential's `id` for a `vc+jwt`. */
export const VC_JWT_PREFIX = "data:application/vc+jwt,";

/**
 * Signs `document` as a compact JWS with the library, under the suite's secret key
 * for `alg`. The payload is the document as `JSON.stringify` writes it, which is
 * not RFC 8785 canonical form: a verifier must not need it to be.
 *
 * ECDSA through Web Crypto draws a fresh nonce, so the `S` is high about half the
 * time. When `wantHigh` is a boolean the signing is repeated until the signature has
 * that form, so that a corpus holds both on purpose and not by chance. 64 draws fail
 * with probability 2^-64.
 */
export async function sign(secretJwk, alg, header, document, wantHigh) {
  const key = await jose.importJWK(secretJwk, alg);
  const payload = new TextEncoder().encode(JSON.stringify(document));
  for (let attempt = 0; attempt < 64; attempt += 1) {
    const token = await new CompactSign(payload).setProtectedHeader(header).sign(key);
    if (!isEcdsa(alg) || wantHigh === undefined || isHighS(alg, signatureOf(token)) === wantHigh) {
      return token;
    }
  }
  throw new Error(`no ${alg} signature of the wanted form in 64 attempts`);
}
