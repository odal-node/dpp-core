/**
 * Judge the credentials and presentations `dpp_vc::jose` signs with an independent
 * JOSE library.
 *
 * `corpus.json` comes from
 * `EMIT_JOSE_CORPUS=1 cargo test -p dpp-vc --lib jose::tests::oracle::emit`. It holds
 * the W3C VC-JOSE-COSE suite's issuance inputs, rows 1 to 5, and credentials and
 * presentations under EdDSA, ES256, ES384 and ES512, all signed with the suite's own
 * keys, and the JWK published for each `kid`. For every token the library has to
 * agree on each of these, and a failure names the check:
 *
 *   compact    the token is a compact JWS the library can read.
 *   kid        the header's `kid` names a published JWK.
 *   alg        the header's `alg` is the key's own, from its `kty` and `crv`, and the
 *              JWK, if it declares one, says the same.
 *   signature  the library verifies the signature under that JWK, accepting only the
 *              key's own algorithm, so `alg: none` and a swapped algorithm are refused.
 *   typ, cty   `vc+jwt` and `vc` for a credential, `vp+jwt` and `vp` for a
 *              presentation, as exact strings, as `dpp_vc::jose` writes them.
 *   payload    the payload is the document that was signed, with key order removed.
 *
 * For a presentation, each `vc+jwt` it envelops is judged the same way under the key
 * its own `kid` names. An envelope of another media type is counted and not judged:
 * a `vc+sd-jwt` is the SD-JWT oracles' to judge and a `vc+cose` the COSE oracle's. Row 3 of the suite embeds an
 * envelope that is not a compact JWS, by the suite's making; the outer token is
 * judged, and the inner one must be unreadable to the library too, which is the
 * evidence that the suite's input, and not this crate, is what is malformed.
 *
 * What this does not answer: whether the credential is a good credential. The
 * library knows no JSON-LD, no VC Data Model, no `@context` rule and no
 * undeclared-member rule. It judges the JOSE envelope: the algorithm, the
 * signature, the two media types and the key.
 *
 * Negative controls run first, because a judge that cannot refuse proves nothing:
 * a flipped signature, a changed payload, a `typ` or a `cty` of the wrong kind, an
 * unknown `kid`, an algorithm that is not the key's, and `alg: none`. Each must fail
 * the named check. The script then reports every failure and exits non-zero if
 * there is one.
 */

import { readFileSync } from "node:fs";
import {
  ALGORITHMS,
  VC_JWT_PREFIX,
  algorithmOf,
  b64u,
  headerOf,
  jose,
  libraryVersion,
  same,
  sign,
  suiteKeys,
} from "./lib.mjs";

const corpusPath = process.argv[2] ?? "target/jose-oracle/corpus.json";
const corpus = JSON.parse(readFileSync(corpusPath, "utf8"));

const TYPES = { vc: ["vc+jwt", "vc"], vp: ["vp+jwt", "vp"] };

/**
 * What the library makes of one token: `[{ check, ok, detail? }]`, never throwing.
 * `expected` is the document the token should carry, when it is known.
 */
async function judgeToken(token, kind, keys, expected) {
  const results = [];
  const note = (check, ok, detail) => results.push({ check, ok, ...(ok ? {} : { detail }) });

  let header;
  try {
    header = jose.decodeProtectedHeader(token);
    if (token.split(".").length !== 3) throw new Error("not three segments");
  } catch (e) {
    note("compact", false, String(e?.message ?? e));
    return { results };
  }
  note("compact", true);

  const [typ, cty] = TYPES[kind];
  note("typ", header.typ === typ, `typ is ${JSON.stringify(header.typ)}, not ${typ}`);
  note("cty", header.cty === cty, `cty is ${JSON.stringify(header.cty)}, not ${cty}`);

  const jwk = typeof header.kid === "string" ? keys[header.kid] : undefined;
  note("kid", jwk !== undefined, `kid ${JSON.stringify(header.kid)} names no published key`);
  if (!jwk) return { results, header };

  // The key's own algorithm, never the header's: a token does not get to choose.
  const keyAlg = algorithmOf(jwk);
  note(
    "alg",
    header.alg === keyAlg && (jwk.alg === undefined || jwk.alg === keyAlg),
    `header alg ${JSON.stringify(header.alg)}, key alg ${keyAlg}, JWK alg ${jwk.alg}`,
  );

  let payload;
  try {
    const key = await jose.importJWK(jwk, keyAlg);
    ({ payload } = await jose.compactVerify(token, key, { algorithms: [keyAlg] }));
    note("signature", true);
  } catch (e) {
    note("signature", false, `${e?.code ?? "error"}: ${String(e?.message ?? e).slice(0, 120)}`);
    return { results, header, keyAlg };
  }

  if (expected !== undefined) {
    let parsed;
    try {
      parsed = JSON.parse(new TextDecoder().decode(payload));
    } catch {
      parsed = undefined;
    }
    note("payload", same(parsed, expected), "the payload is not the document that was signed");
  }
  return { results, header, keyAlg };
}

/** Every `vc+jwt` a presentation envelops, with its position; the rest, counted. */
function envelopesOf(document) {
  const found = { tokens: [], other: [] };
  for (const [index, entry] of (document.verifiableCredential ?? []).entries()) {
    const id = typeof entry?.id === "string" ? entry.id : "";
    if (id.startsWith(VC_JWT_PREFIX)) found.tokens.push({ index, token: id.slice(VC_JWT_PREFIX.length) });
    else found.other.push({ index, id: id.slice(0, id.indexOf(",") > 0 ? id.indexOf(",") : 40) });
  }
  return found;
}

// ── negative controls ────────────────────────────────────────────────────────

const failures = [];
const fail = (message) => failures.push(message);

const { keys: suite, published } = suiteKeys();
const credentialDocument = { "@context": ["https://www.w3.org/ns/credentials/v2"], type: ["VerifiableCredential"] };

async function controls() {
  let run = 0;
  const expectFailing = async (label, token, kind, expectedChecks, document = credentialDocument) => {
    const { results } = await judgeToken(token, kind, published, document);
    const failed = results.filter((r) => !r.ok).map((r) => r.check);
    run += 1;
    for (const check of expectedChecks) {
      if (!failed.includes(check)) fail(`control "${label}" was not refused at ${check}; failed: [${failed}]`);
    }
    if (expectedChecks.length === 0 && failed.length > 0) fail(`control "${label}" must pass; failed: [${failed}]`);
  };

  for (const [alg, k] of Object.entries(suite)) {
    const good = { alg, kid: k.id, typ: "vc+jwt", cty: "vc" };
    const token = await sign(k.secretJwk, alg, good, credentialDocument);
    await expectFailing(`${alg}: the library's own token`, token, "vc", []);

    const [h, p, s] = token.split(".");
    const flipped = s.slice(0, 10) + (s[10] === "A" ? "B" : "A") + s.slice(11);
    await expectFailing(`${alg}: a flipped signature`, `${h}.${p}.${flipped}`, "vc", ["signature"]);
    const other = b64u(JSON.stringify({ ...credentialDocument, id: "urn:other" }));
    await expectFailing(`${alg}: a changed payload`, `${h}.${other}.${s}`, "vc", ["signature"]);
    await expectFailing(
      `${alg}: another document than the one signed`,
      token,
      "vc",
      ["payload"],
      { ...credentialDocument, id: "urn:other" },
    );
    await expectFailing(`${alg}: typ of JWT`, await sign(k.secretJwk, alg, { ...good, typ: "JWT" }, credentialDocument), "vc", ["typ"]);
    await expectFailing(`${alg}: typ as a media type`, await sign(k.secretJwk, alg, { ...good, typ: "application/vc+jwt" }, credentialDocument), "vc", ["typ"]);
    await expectFailing(`${alg}: cty of vp`, await sign(k.secretJwk, alg, { ...good, cty: "vp" }, credentialDocument), "vc", ["cty"]);
    await expectFailing(`${alg}: a credential judged as a presentation`, token, "vp", ["typ", "cty"]);
    await expectFailing(`${alg}: an unknown kid`, await sign(k.secretJwk, alg, { ...good, kid: "https://nowhere.example/k" }, credentialDocument), "vc", ["kid"]);
    await expectFailing(`${alg}: no kid`, await sign(k.secretJwk, alg, { alg, typ: "vc+jwt", cty: "vc" }, credentialDocument), "vc", ["kid"]);
  }

  // The kid of one algorithm's key on a token of another's: the algorithm is not the
  // key's, and the signature does not verify under it.
  const es256 = await sign(suite.ES256.secretJwk, "ES256", { alg: "ES256", kid: suite.ES384.id, typ: "vc+jwt", cty: "vc" }, credentialDocument);
  await expectFailing("ES256 token under the ES384 key", es256, "vc", ["alg", "signature"]);
  const none = `${b64u(JSON.stringify({ alg: "none", kid: suite.ES256.id, typ: "vc+jwt", cty: "vc" }))}.${b64u(JSON.stringify(credentialDocument))}.`;
  await expectFailing("alg none", none, "vc", ["alg", "signature"]);
  await expectFailing("not a JWS", "not.a.token", "vc", ["compact"]);
  return run;
}

// ── the corpus ───────────────────────────────────────────────────────────────

/** `stats[check][group]`, where group is an algorithm or `all`. */
const stats = {};
const count = (check, alg, ok) => {
  for (const group of [alg, "all"]) {
    stats[check] ??= {};
    stats[check][group] ??= { pass: 0, fail: 0 };
    stats[check][group][ok ? "pass" : "fail"] += 1;
  }
};
const tokens = { outer: 0, enveloped: 0, skipped: 0 };

function record(id, role, result, alg) {
  for (const { check, ok, detail } of result.results) {
    count(check, alg, ok);
    if (!ok) fail(`${id} (${role}): ${check}: ${detail}`);
  }
}

async function judgeCorpus() {
  for (const c of corpus.cases) {
    const outer = await judgeToken(c.token, c.kind, corpus.keys, c.document);
    if (outer.header?.alg !== c.alg) fail(`${c.id}: the corpus says ${c.alg}, the header says ${outer.header?.alg}`);
    tokens.outer += 1;
    record(c.id, "outer", outer, c.alg);

    if (c.kind !== "vp") continue;
    const { tokens: inner, other } = envelopesOf(c.document);
    tokens.skipped += other.length;
    for (const { index, token } of inner) {
      const label = `${c.id}[${index}]`;
      if (c.innerMalformedBySuite) {
        // The suite's own input: the library must find it unreadable too.
        const result = await judgeToken(token, "vc", corpus.keys, undefined);
        const unreadable = result.results.some((r) => r.check === "compact" && !r.ok);
        count("suite-malformed", c.alg, unreadable);
        if (!unreadable) fail(`${label}: the suite's malformed envelope was read by the library`);
        continue;
      }
      const alg = headerOf(token).alg;
      tokens.enveloped += 1;
      record(label, "enveloped", await judgeToken(token, "vc", corpus.keys, undefined), alg);
    }
  }
}

console.log(`node ${process.version}`);
console.log(`jose ${libraryVersion}`);
const controlsRun = await controls();
console.log(`controls: ${controlsRun} run, ${failures.length === 0 ? "each refused or accepted as it must be" : "FAILED"}`);
await judgeCorpus();

const columns = ["all", ...Object.keys(ALGORITHMS)];
console.log(`\ncorpus ${corpusPath}: ${corpus.cases.length} cases`);
console.log(`tokens judged: ${tokens.outer} outer, ${tokens.enveloped} enveloped vc+jwt; ${tokens.skipped} envelopes of another media type not judged`);
console.log("\ncheck".padEnd(20) + columns.map((c) => c.padStart(10)).join(""));
for (const [check, byGroup] of Object.entries(stats)) {
  console.log(
    check.padEnd(19) +
      columns
        .map((c) => (byGroup[c] ? `${byGroup[c].pass}/${byGroup[c].pass + byGroup[c].fail}` : "-").padStart(10))
        .join(""),
  );
}
console.log("(passed/judged; the algorithm of an enveloped credential is its own)");

// A corpus that lost an algorithm, a suite row or every envelope would still agree
// with itself, so the corpus is held to its shape as well.
// The payload is checked on outer tokens only, so this asks for an outer token per
// algorithm: an envelope signed under it does not stand in for one.
for (const alg of Object.keys(ALGORITHMS)) {
  if (!(stats.payload?.[alg]?.pass > 0)) fail(`no ${alg} credential or presentation was judged`);
}
for (const row of [1, 2, 3, 4, 5]) {
  if (!corpus.cases.some((c) => c.id === `suite/row-${row}`)) fail(`suite row ${row} is not in the corpus`);
}
if (tokens.enveloped === 0) fail("no enveloped vc+jwt was judged");

if (failures.length > 0) {
  console.log(`\n${failures.length} failure(s):`);
  for (const f of failures) console.log(`  ${f}`);
  process.exit(1);
}
console.log("\nevery token judged agrees");
