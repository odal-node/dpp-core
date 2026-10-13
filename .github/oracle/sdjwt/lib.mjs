/**
 * Shared by `judge.mjs` and `issue.mjs`: the independent SD-JWT library, the
 * cryptography it is handed, and the few helpers both scripts need.
 *
 * The library takes its signature check, its hash and its salt source as
 * callbacks, and never touches a key itself. So everything below that decides
 * whether a signature is valid is Node's own `crypto`, written here, not code from
 * the library. What the library contributes is RFC 9901's processing: the
 * Disclosures, the digests, the clause 7.1 rejections. Say that plainly wherever
 * this oracle is cited: the signature is checked by Node, in this file.
 *
 * The library is looked up in `SDJWT_ORACLE_MODULES`, or else next to this file
 * (where CI installs it). `createRequire` rather than a bare import because ESM
 * ignores `NODE_PATH`, and the pinned install may sit outside the repository.
 */

import {
  createHash,
  createPrivateKey,
  createPublicKey,
  randomBytes,
  sign as cryptoSign,
  verify as cryptoVerify,
} from "node:crypto";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
export const modulesDir = process.env.SDJWT_ORACLE_MODULES ?? path.join(here, "node_modules");
const require = createRequire(path.join(modulesDir, "x.js"));

export const { SDJwtInstance } = require("@sd-jwt/core");
export const libraryVersion = require("@sd-jwt/core/package.json").version;

const NODE_HASH = { "sha-256": "sha256", "sha-384": "sha384", "sha-512": "sha512" };

/**
 * The digest function. SHA-512 is supported on purpose: this crate refuses it,
 * and a hasher that could not compute it would make the library refuse for the
 * wrong reason, hiding a disagreement the judge should show.
 */
export function hasher(data, alg) {
  const name = NODE_HASH[alg];
  if (!name) throw new Error(`the judge has no hash for ${alg}`);
  return createHash(name).update(typeof data === "string" ? data : Buffer.from(data)).digest();
}

const b64u = (bytes) => Buffer.from(bytes).toString("base64url");

/** A verifier for one JWK: Ed25519 (EdDSA) or P-256 (ES256, raw r||s). */
export function verifierFor(jwk) {
  const key = createPublicKey({ key: jwk, format: "jwk" });
  const options =
    jwk.kty === "EC" ? { key, dsaEncoding: "ieee-p1363" } : { key };
  const digest = jwk.kty === "EC" ? "sha256" : null;
  return (data, signature) =>
    cryptoVerify(digest, Buffer.from(data), options, Buffer.from(signature, "base64url"));
}

/** Checks a Key Binding JWT against the `cnf` key the Issuer put in the payload. */
export function kbVerifier(data, signature, payload) {
  const jwk = payload?.cnf?.jwk;
  if (!jwk) return false;
  return verifierFor(jwk)(data, signature);
}

/** An instance that verifies `jwk`'s signatures: `alg` is `EdDSA` or `ES256`. */
export function verifyingInstance(jwk, alg) {
  return new SDJwtInstance({
    hasher,
    hashAlg: "sha-256",
    signAlg: alg,
    verifier: verifierFor(jwk),
    kbVerifier,
  });
}

/**
 * What the library makes of a token: `{ accepts: true, payload }` or
 * `{ accepts: false, error }`. Never throws, so one odd token cannot stop the run.
 */
export async function judge(instance, token, options = {}) {
  try {
    const { payload } = await instance.verify(token, options);
    return { accepts: true, payload };
  } catch (e) {
    return { accepts: false, error: String(e?.message ?? e).slice(0, 200) };
  }
}

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

/**
 * A key pair for the reverse direction, derived from a fixed seed so the issued
 * file is reproducible. The PKCS#8 prefix is the fixed 16 bytes that precede a
 * raw Ed25519 seed (RFC 8410).
 */
export function ed25519FromSeed(seedText) {
  const seed = createHash("sha256").update(seedText).digest();
  const prefix = Buffer.from("302e020100300506032b657004220420", "hex");
  const privateKey = createPrivateKey({
    key: Buffer.concat([prefix, seed]),
    format: "der",
    type: "pkcs8",
  });
  const publicJwk = createPublicKey(privateKey).export({ format: "jwk" });
  return { privateKey, publicJwk, sign: (data) => b64u(cryptoSign(null, Buffer.from(data), privateKey)) };
}

/**
 * Every `_sd` array in a token: in the signed payload, and in the values of the
 * Disclosures, where a recursive Disclosure carries its own. Read without
 * checking anything.
 */
export function sdArrays(token) {
  const found = [];
  const walk = (value) => {
    if (Array.isArray(value)) {
      value.forEach(walk);
    } else if (value && typeof value === "object") {
      for (const [key, inner] of Object.entries(value)) {
        if (key === "_sd" && Array.isArray(inner)) found.push(inner);
        walk(inner);
      }
    }
  };
  const [jwt, ...rest] = token.split("~");
  walk(JSON.parse(Buffer.from(jwt.split(".")[1], "base64url")));
  for (const segment of rest) {
    // A Key Binding JWT has dots; a Disclosure, being base64url alone, never does.
    if (segment === "" || segment.includes(".")) continue;
    const disclosure = JSON.parse(Buffer.from(segment, "base64url"));
    walk(disclosure.at(-1));
  }
  return found;
}

/**
 * Whether an `_sd` array is in strictly ascending order. RFC 9901 clause 4.2.4.1
 * requires the Issuer to hide the original order of the claims, and names sorting
 * the digests alphanumerically as a way to do it: an order that is a function of
 * the salted digests alone cannot carry the order the claims were in. The
 * original order cannot be seen. That the array is sorted can, by anyone.
 * Digests are ASCII, so this order is the byte order a Rust `sort` gives.
 */
export const ascending = (digests) => digests.every((d, i) => i === 0 || digests[i - 1] < d);

/** A deterministic generator (mulberry32), so a seed names one corpus. */
export function prng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export { b64u, randomBytes };
