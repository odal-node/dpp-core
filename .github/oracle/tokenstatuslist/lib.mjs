/**
 * Shared by `judge.mjs` and `issue.mjs`: the independent Token Status List
 * library, the cryptography it is handed, and the few helpers both scripts need.
 *
 * The library never touches a key unless it is handed a callback. The scripts here
 * hand it none for verifying: a token is read with `decodeStatusListJwt`, its claims
 * are checked with `verifyStatusListJwtClaims`, and its list is read with
 * `StatusList`. So everything below that decides whether a signature is valid is
 * Node's own `crypto`, written in this file, and not code from the library. What the
 * library contributes is the draft's processing: the zlib list, the bit order, the
 * status values, `sub` against `uri`, `exp`. Say that plainly wherever this oracle
 * is cited.
 *
 * Signing, in `issue.mjs`, is the one place the library is handed a callback, and it
 * is again Node's `crypto` that signs.
 *
 * The library is looked up in `STATUS_LIST_ORACLE_MODULES`, or else next to this
 * file (where CI installs it). `createRequire` and a path, rather than a bare
 * import, because ESM ignores `NODE_PATH` and the pinned install may sit outside
 * the repository.
 */

import {
  createPrivateKey,
  createPublicKey,
  generateKeyPairSync,
  sign as cryptoSign,
  verify as cryptoVerify,
} from "node:crypto";
import { createRequire } from "node:module";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
export const repoRoot = path.resolve(here, "../../..");
export const outDir = path.join(repoRoot, "target/status-list-oracle");
export const modulesDir = process.env.STATUS_LIST_ORACLE_MODULES ?? path.join(here, "node_modules");

const require = createRequire(path.join(modulesDir, "x.js"));
export const library = await import(pathToFileURL(require.resolve("@owf/token-status-list")).href);
export const libraryVersion = require("@owf/token-status-list/package.json").version;

/** The hash each JWS algorithm signs with (RFC 7518 clause 3.4); EdDSA signs the message itself. */
const DIGEST = { ES256: "sha256", ES384: "sha384", ES512: "sha512", EdDSA: null };

const b64u = (bytes) => Buffer.from(bytes).toString("base64url");

/** Whether `signature` (base64url) is `alg`'s signature over `input` under `jwk`. Node decides, not the library. */
export function signatureHolds(alg, jwk, input, signature) {
  if (!(alg in DIGEST)) return false;
  const key = createPublicKey({ key: jwk, format: "jwk" });
  const options = alg === "EdDSA" ? key : { key, dsaEncoding: "ieee-p1363" };
  return cryptoVerify(DIGEST[alg], Buffer.from(input), options, Buffer.from(signature, "base64url"));
}

/** A key pair of `alg`, as a public JWK and a signer over bytes. Generated, never committed. */
export function generateSigner(alg) {
  const curve = { ES256: "P-256", ES384: "P-384", ES512: "P-521" }[alg];
  const { publicKey, privateKey } =
    alg === "EdDSA" ? generateKeyPairSync("ed25519") : generateKeyPairSync("ec", { namedCurve: curve });
  const publicJwk = publicKey.export({ format: "jwk" });
  const signingKey = createPrivateKey(privateKey.export({ format: "pem", type: "pkcs8" }));
  const options = alg === "EdDSA" ? signingKey : { key: signingKey, dsaEncoding: "ieee-p1363" };
  return {
    alg,
    publicJwk,
    sign: (input) => b64u(cryptoSign(DIGEST[alg], Buffer.from(input), options)),
  };
}

export function readJson(file) {
  return JSON.parse(readFileSync(file, "utf8"));
}

export function writeJson(file, value) {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, JSON.stringify(value));
}

/** The vectors of Appendix C of the draft, as `dpp-vc` keeps them: `[{ name, bits, statuses, lst }]`. */
export function draftVectors() {
  const file = path.join(repoRoot, "crates/dpp-vc/src/token_status_list/draft_21_appendix_c.txt");
  const vectors = [];
  for (const line of readFileSync(file, "utf8").split("\n")) {
    if (line === "" || line.startsWith("#")) continue;
    const [kind, ...rest] = line.split(" ");
    if (kind === "vector") vectors.push({ name: rest[0], bits: Number(rest[2]), statuses: [], lst: "" });
    else if (kind === "status") vectors.at(-1).statuses.push([Number(rest[0]), Number(rest[1])]);
    else if (kind === "lst") vectors.at(-1).lst = rest[0];
    else throw new Error(`a line nothing here reads: ${line}`);
  }
  return vectors;
}
