/**
 * Has the independent implementation issue Status List Tokens, for `dpp-vc` to read.
 *
 * Writes `target/status-list-oracle/issued.json`. `CHECK_STATUS_LIST_ORACLE_ISSUED=...
 * cargo test -p dpp-tests --test status_list_oracle_corpus` then verifies each token
 * with `StatusListToken::verify`, and reads every entry the file lists with `status`.
 * Writing is the library's (`signStatusListJwt`, over a `StatusList` it built); the
 * signature is Node's `crypto`, handed to it as the callback. The keys are generated
 * on each run, so no key material is committed, and ECDSA signatures differ from run
 * to run. The lists do not: their values come from a fixed formula.
 *
 * What it covers: every width the draft allows (1, 2, 4, 8 bits) under every signature
 * algorithm this crate verifies (EdDSA, ES256, ES384, ES512); a list with every kind
 * of value in it, registered, application-specific and not registered; claims the
 * draft makes optional, left out and put in; and the draft's own list of 2^20 entries.
 */

import path from "node:path";

import { draftVectors, generateSigner, library, libraryVersion, outDir, writeJson } from "./lib.mjs";

const { StatusList, signStatusListJwt } = library;

const ISSUED_AT = 1_789_000_000;
const NOW = ISSUED_AT + 60;
const URI = "https://status.example/statuslists/1";

/** `[[idx, value], ...]` for every entry of `list`, or only for `only` when it is given. */
function entriesOf(list, only) {
  const idxs = only ?? Array.from({ length: list.totalStatuses }, (_, idx) => idx);
  return idxs.map((idx) => [idx, list.getStatus(idx)]);
}

async function issue({ name, signer, list, payload, uri = URI, only }) {
  const { jwt } = await signStatusListJwt({
    statusList: list,
    payload: { sub: uri, iat: ISSUED_AT, ...payload },
    signer: { method: "jwk", publicJwk: signer.publicJwk, alg: signer.alg },
    signJwsCallback: async (_signer, { signingInputBytes }) => ({
      signature: signer.sign(signingInputBytes),
      signerJwk: signer.publicJwk,
    }),
  });
  return {
    name,
    alg: signer.alg,
    jwk: signer.publicJwk,
    token: jwt,
    uri,
    bits: list.getBitsPerStatus(),
    entries: list.totalStatuses,
    expect: entriesOf(list, only),
  };
}

const cases = [];

for (const alg of ["EdDSA", "ES256", "ES384", "ES512"]) {
  const signer = generateSigner(alg);
  for (const bits of [1, 2, 4, 8]) {
    // Every value that fits, cycling, so that a width that read a neighbour's bits
    // would show. 64 entries is a whole number of bytes at every width.
    const distinct = 2 ** bits;
    const values = Array.from({ length: 64 }, (_, idx) => (idx * 7 + bits) % distinct);
    cases.push(
      await issue({
        name: `${alg} ${bits}-bit, every value`,
        signer,
        list: new StatusList(values, bits),
        payload: { exp: ISSUED_AT + 86_400, ttl: 43_200 },
      }),
    );
  }
}

const signer = generateSigner("ES256");
const small = new StatusList(
  Array.from({ length: 16 }, (_, idx) => (idx % 3 === 0 ? 1 : idx % 3 === 1 ? 2 : 0)),
  2,
);
cases.push(
  await issue({ name: "no exp and no ttl", signer, list: small, payload: {} }),
  await issue({
    name: "an iss, as the draft's own example has",
    signer,
    list: small,
    payload: { iss: "https://status.example", exp: ISSUED_AT + 600, ttl: 600 },
  }),
  await issue({
    name: "an aggregation_uri",
    signer,
    list: new StatusList(Array.from({ length: 16 }, () => 0), 1, "https://status.example/aggregation"),
    payload: { exp: ISSUED_AT + 600 },
  }),
  await issue({ name: "a fractional ttl", signer, list: small, payload: { ttl: 0.5 } }),
);

// The draft's own list of 2^20 entries, the 1-bit one of Appendix C.1, issued by the
// library and read here.
const [c1] = draftVectors();
const big = new StatusList(new Array(2 ** 20).fill(0), 1);
for (const [idx, value] of c1.statuses) big.setStatus(idx, value);
cases.push(
  await issue({
    name: "the draft's 2^20 entries (C.1)",
    signer,
    list: big,
    payload: { exp: ISSUED_AT + 86_400, ttl: 3600 },
    only: [...c1.statuses.map(([idx]) => idx), 1, 2, 1992, 1994, 25_461, 1_048_575],
  }),
);

writeJson(path.join(outDir, "issued.json"), { now: NOW, cases });
console.log(`@owf/token-status-list ${libraryVersion} issued ${cases.length} tokens`);
