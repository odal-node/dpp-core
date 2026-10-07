# Standards Register

One row per IETF, W3C, GS1, IDTA and ETSI specification this repository cites.
ISO/IEC and IEC standards are not yet covered; that is tracked in #392. Each row
records:

- which revision is cited;
- the specification's status when it was last read, and the date of that read;
- where the code uses it;
- what evidence backs that use;
- whether conformance is claimed.

**Law is not recorded here.** Acts and their bindings live in
`crates/dpp-domain/instruments/`, and the CEN/CLC harmonised standards, whose
status is a fact about the Official Journal, are covered in
[`regulatory/CONFORMITY.md`](../regulatory/CONFORMITY.md).

## How this register is kept true

- **IETF rows are machine-checked.** `crates/dpp-tests/tests/standard_citations.rs`
  finds every `RFC NNNN` and `draft-ietf-…` cited in the repository. It fails
  when:
  - a cited specification has no row;
  - an obsoleted specification is cited without a `kept:` reason in its row;
  - a draft is cited without its revision number, or its row claims
    conformance;
  - a row is cited nowhere.
- **Every row, in both tables, carries a status and the date it was read.** The
  same test fails on an empty `Status as read`, or a `Read` that is not a
  `YYYY-MM-DD` date.
- **A conformance claim is held to its evidence.** The same test fails on a
  `Yes` that names no repository path, that names one which does not exist, or
  whose row offers nothing but unit tests as evidence.
- **The other rows are not checked against the code.** W3C, GS1, IDTA and ETSI
  identifiers take too many shapes to match reliably, so nothing checks that
  they are still cited. The re-read below is the only check on that.
- **Every status is re-read at release.** The Pre-Release Checklist in
  [`governance/RELEASE.md`](../governance/RELEASE.md) re-reads each row at its
  source and updates the date. The gate proves that every citation has a status
  somebody read and dated. It cannot prove that status is still current.

## Claiming conformance

A conformance claim lives in one place: the `Conformance claimed` cell of the
specification's row. No other document restates it;
[`README.md`](../../README.md), [`regulatory/CONFORMITY.md`](../regulatory/CONFORMITY.md)
and [`project/BLUEPRINT.md`](../project/BLUEPRINT.md) point here instead. Code
here *implements* the parts of a specification it uses, and that is not a claim.

A cell may start with `Yes` only if it says, in the cell itself:

1. the **conformance class** the specification defines, such as a conforming
   issuer implementation;
2. the **scope**: the features and options covered, for example "compact
   serialisation, no key binding";
3. the **known deviations**. A claim with a deviation it does not list is not
   made;
4. that the claim is **self-declared**, since no body certifies these;
5. **evidence that is not circular**, each piece named as a repository path: the
   specification's own test vectors, an official or independent test suite, or an
   independent implementation used as an oracle.

Unit tests written against this repository's own reading of a specification can
never carry a claim alone, because the code and its tests would share any
misreading.

`standard_citations.rs` checks the form of a claim, not its truth. Whether the
named evidence is not circular, and whether the class, scope and deviations are
right, is a reviewer's call. A cell that reads `No` makes no claim.

## IETF

| Spec | Title | Status as read | Read | Implemented in | Evidence | Conformance claimed |
|---|---|---|---|---|---|---|
| RFC 3339 | Date and Time on the Internet: Timestamps | Proposed Standard; updated by RFC 9557 | 2026-09-29 | `dpp-vc` snapshot validity instants | Unit tests | No |
| RFC 3986 | Uniform Resource Identifier (URI): Generic Syntax | Internet Standard; updated by RFC 7320, RFC 8820 | 2026-09-29 | `dpp-digital-link` path encoding; `dpp-aas` asset URIs; `dpp-vc` `did:web` paths | Unit tests | No |
| RFC 4151 | The 'tag' URI Scheme | Informational | 2026-09-29 | `dpp-vc` SD-JWT VC `vct` | Unit tests | No |
| RFC 7515 | JSON Web Signature (JWS) | Proposed Standard | 2026-09-29 | `dpp-crypto` JWS and JAdES headers; `dpp-domain` seal envelope | Unit tests; JAdES output checked by the European Commission's DSS in `jades-oracle.yml` | No |
| RFC 7517 | JSON Web Key (JWK) | Proposed Standard | 2026-10-05 | `dpp-crypto` published JWK (`kid`, `alg`); `dpp-vc` DID document `publicKeyJwk` and SD-JWT VC issuer metadata | Unit tests | No |
| RFC 7638 | JSON Web Key (JWK) Thumbprint | Proposed Standard | 2026-10-05 | `dpp-crypto` key identity; `dpp-vc` verification-method identifiers | The thumbprint RFC 8037 appendix A.3 publishes for its example key (`thumbprint_tests.rs`) | No |
| RFC 7519 | JSON Web Token (JWT) | Proposed Standard; updated by RFC 7797, RFC 8725 | 2026-09-29 | `dpp-vc` SD-JWT VC validity claims | Unit tests | No |
| RFC 8032 | Edwards-Curve Digital Signature Algorithm (EdDSA) | Informational. Six verified errata, which correct the Python reference code, the scalar's range in clause 3.1, a notation in the group equation, a cross-reference and two typos. None changes an Ed25519 operation or a vector. Four more are held for a document update | 2026-10-07 | `dpp-crypto` JWS signing and strict verification, through `ed25519-dalek` | The five Ed25519 test vectors of clause 7.1, including the 1023-byte message, run on every build against the `ed25519-dalek` version locked in `Cargo.lock`: the public key derived from the secret, the deterministic signature the RFC prints, and acceptance by strict verification; unit tests | Yes. **Class:** RFC 8032 defines no conformance classes, so the claim is for the pure Ed25519 scheme of clause 5.1: key derivation, signing and verification. **Scope:** nothing in this repository implements the curve, so this is the claim that `ed25519-dalek`, at the version locked here, derives, signs and verifies as the RFC prints, used through signing and `verify_strict`. **Known deviations:** `verify_strict` is stricter than clause 5.1.7's check. It also refuses a signature whose nonce point or whose key is of small order, which the clause does not, and it compares the cofactorless equation, which the clause allows but does not require. Verification therefore accepts a subset of what the RFC permits. **Not claimed:** Ed25519ctx, Ed25519ph, Ed448 and Ed448ph, and resistance to side channels. **Self-declared:** no body certifies an Ed25519 implementation. **Evidence:** `crates/dpp-crypto/src/jws/rfc8032_vector_tests.rs` |
| RFC 8037 | CFRG ECDH and Signatures in JOSE | Proposed Standard; **updated by RFC 9864**, which deprecates the JOSE `alg` value `EdDSA`. `dpp-crypto` still writes it, for the reason in the RFC 9864 row | 2026-10-05 | `dpp-crypto` `alg`/`crv` names; `dpp-vc` JWK `kty` | Unit tests | No |
| RFC 8410 | Algorithm Identifiers for Ed25519, Ed448, X25519, and X448 for Use in the Internet X.509 PKI | Proposed Standard; updated by RFC 9295 | 2026-09-29 | `dpp-crypto` JAdES oracle artefact (PKCS#8 key) | Artefact validated in `jades-oracle.yml` | No |
| RFC 8725 | JSON Web Token Best Current Practices | Best Current Practice; updates RFC 7519 | 2026-10-05 | `dpp-vc` VC-JWT verification requires `typ: vc+jwt`, the explicit typing of §3.11 | Unit tests | No |
| RFC 8785 | JSON Canonicalization Scheme (JCS) | Informational. Two verified errata: a cross-reference in clause 3.2.2.2, and erratum 7920, which says a parser should refuse `-0`, since it is written as `0` | 2026-10-07 | Signing and content-binding canonical form across `dpp-crypto`, `dpp-domain`, `dpp-rules`, `dpp-calc`, `dpp-vc` | The RFC's own examples, run on every build through `serde_jcs` at the version locked in `Cargo.lock`: every row of Appendix B's number table from its IEEE 754 bit pattern, and its integer rows from their text; the worked example of clauses 3.2.2 to 3.2.4, byte for byte; and the data for the UTF-16 property sort; unit tests | Yes. **Class:** RFC 8785 defines no conformance classes, so the claim is for a canonicaliser, which writes the one JCS byte form of a JSON value. **Scope:** `canonicalize`, over a `serde_json::Value`, delegated to `serde_jcs`. **Known deviations:** `-0` is written as `0` and is not refused, which erratum 7920 says a parser should do; a number that a double cannot hold is rounded to the nearest double without a warning, as clause 3.2.2.3 requires; a JSON text with a repeated member name is read by `serde_json` before it reaches the canonicaliser, which keeps the last of them and does not refuse it. **Not claimed:** Appendix B's NaN and Infinity rows, which a `Value` cannot hold and so cannot reach the canonicaliser; the larger number file the RFC's author publishes separately, which is not run; and validation of the input as I-JSON. **Self-declared:** no body certifies a JCS implementation. **Evidence:** `crates/dpp-crypto/src/jws/rfc8785_vector_tests.rs` |
| RFC 9106 | Argon2 Memory-Hard Function for Password Hashing and Proof-of-Work Applications | Informational | 2026-09-29 | `dpp-crypto` keystore key derivation | Delegated to `argon2`; frozen vectors produced by `argon2` 0.5.3, **not** the RFC's own vectors | No |
| RFC 9110 | HTTP Semantics | Internet Standard | 2026-09-29 | `dpp-digital-link` `Accept` q-value parsing | Unit tests | No |
| RFC 9278 | JWK Thumbprint URI | Proposed Standard | 2026-10-05 | `dpp-crypto` thumbprint URI; `dpp-vc` verification-method fragments, JWK `kid`, and the SD-JWT VC `kid` | Unit tests | No |
| RFC 9562 | Universally Unique IDentifiers (UUIDs) | Proposed Standard | 2026-09-29 | `dpp-domain` passport id, UUIDv7 layout | Delegated to `uuid`; serial-derivation regression tests | No |
| RFC 9864 | Fully-Specified Algorithms for JSON Object Signing and Encryption (JOSE) and CBOR Object Signing and Encryption (COSE) | Proposed Standard; updates RFC 7518, RFC 8037, RFC 9053 | 2026-10-05 | `dpp-crypto` accepts `alg: Ed25519` when verifying and still writes the deprecated `EdDSA`, under the exception in §4.4 for documented operational requirements: the European Commission's DSS 6.5 maps only `EdDSA` for JOSE, and the W3C VC-JOSE-COSE test suite signs its fixtures with it. Re-read both at each release; the emitted name changes when both have moved | Unit tests | No |
| RFC 9901 | Selective Disclosure for JSON Web Tokens | Proposed Standard. It updates and obsoletes nothing and nothing updates or obsoletes it | 2026-10-07 | `dpp-crypto::sd_jwt`; `dpp-vc::sd_jwt_vc` | The RFC's own examples, run on every build: the Disclosure of clause 4.2.1 and its three further encodings, by byte and by digest; the complete SD-JWTs of Appendix A.1, A.3 and A.4, with decoy digests, recursive Disclosures and a Key Binding JWT, processed to the payload the RFC prints for each; sections 6.1 to 6.3, three structures of one claim, with every digest the RFC prints; and a Holder's presentation rebuilt from A.3's issued token to the one the RFC prints; unit tests | Yes. **Class:** the roles the RFC defines, Issuer, Holder and Verifier, each in part. **Scope:** `dpp-crypto::sd_jwt` only, with SHA-256 as the one hash algorithm; Disclosures for object properties, including recursive ones; no Key Binding, no array elements and no decoys issued. As Issuer, creating a Disclosure and its digest (clauses 4.2.1 and 4.2.3). As Holder, presenting by digest (clause 7.2). As Verifier, the Disclosure and digest processing of clause 7.1, steps 1 and 3 to 5, which include refusing a repeated digest, a Disclosure no digest refers to, and a claim that collides with one in the clear. **Known deviations:** array-element Disclosures (clause 4.2.2) are not read, so a placeholder for an array element survives into the processed payload where step 3.d removes it, and a two-element Disclosure is refused; decoy digests are accepted on verification and never issued; a Key Binding JWT is carried and ignored, never validated (clauses 4.3 and 7.3); the JWS JSON Serialization of clause 8 is not read; step 2 (the signature, the algorithm and the issuer) and step 6 (the validity claims) belong to the caller, so the RFC's own signatures, which are ES256, are not verified here. **Not claimed:** how an Issuer builds the payload around the digests, including clause 4.2.4.1's hiding of claim order, which this repository's own tests check and no vector does; and interoperability with another implementation's tokens beyond the RFC's own examples. **Self-declared:** no body certifies an SD-JWT implementation. **Evidence:** `crates/dpp-crypto/src/sd_jwt/rfc9901_vector_tests.rs`, `crates/dpp-crypto/src/sd_jwt/rfc9901_example_tests.rs` and `crates/dpp-crypto/src/sd_jwt/rfc9901_examples.rs` |
| `draft-ietf-oauth-sd-jwt-vc-19` | SD-JWT-based Verifiable Digital Credentials (SD-JWT VC) | Internet-Draft; revision 19 is current, intended as a Proposed Standard, not yet an RFC | 2026-09-29 | `dpp-vc::sd_jwt_vc` | Unit tests | No |

## Other specifications

| Specification | Revision cited | Status as read | Read | Implemented in | Evidence | Conformance claimed |
|---|---|---|---|---|---|---|
| W3C Verifiable Credentials Data Model | v2.0 | W3C Recommendation, 15 May 2025 | 2026-09-29 | `dpp-vc::credential` (`DppAccessCredential`) | Unit tests; `access_gatekeeping.rs` | No |
| W3C Decentralized Identifiers (DIDs) | v1.0 | W3C Recommendation, 19 July 2022. v1.1 is a Candidate Recommendation Snapshot (5 March 2026) that defines its key properties by reference to Controlled Identifiers v1.0 | 2026-10-05 | `dpp-vc` DID document builder; DID syntax in `dpp-rules::common::identifier` | Unit tests | No |
| W3C Controlled Identifiers | v1.0 | W3C Recommendation, 15 May 2025 | 2026-10-05 | `dpp-vc` DID document: `JsonWebKey` verification methods, `cid/v1` context; `dpp-crypto` key resolution (the binding checks of §3.3) | Unit tests | No |
| W3C Securing Verifiable Credentials using JOSE and COSE | v1.0 | W3C Recommendation, 15 May 2025 | 2026-10-05 | `dpp-vc` VC-JWT `kid` (key discovery), `typ: vc+jwt` and `cty: vc` | Unit tests | No |
| `did:web` Method Specification | Unversioned | A W3C Credentials Community Group document whose own `specStatus` is `unofficial`. **Not a W3C standard** | 2026-09-29 | `dpp-vc` DID document builder | Unit tests | No |
| W3C Bitstring Status List | v1.0 | W3C Recommendation, 15 May 2025 | 2026-10-05 | `dpp-vc::status_list`; `credential::check_revocation`, which answers only an entry whose `statusPurpose` is `revocation` | Unit tests | No |
| W3C JSON-LD | 1.1 | W3C Recommendation, 16 July 2020 | 2026-09-29 | `dpp-vc::jsonld` context | Unit tests; `jsonld_context.rs` | No |
| JSON Schema | Draft-07 | Later drafts published (2019-09, 2020-12). The product group schemas pin Draft-07 | 2026-09-29 | `crates/dpp-domain/schemas/` | Every schema compiled and exercised by the schema tests | No |
| GS1 Digital Link URI Syntax | **Not established** | EN 18219:2026 clause 6.3.2 names 1.6.0:2022. The parser has been diffed against no revision. See [`regulatory/CONFORMITY.md`](../regulatory/CONFORMITY.md) | 2026-09-29 | `dpp-digital-link` | Unit tests; CSET 82 membership judged by GS1's Barcode Syntax Engine in `gs1-oracle.yml` | No |
| IDTA Asset Administration Shell Part 1: Metamodel | IDTA-01001-3-0 | IDTA's specification page lists **IDTA-01001-3-2** as the current release | 2026-09-29 | `dpp-aas`, carrying this library's own `urn:odal-node:*` semantics | Unit tests; environments run through an external AAS implementation in `aas-oracle.yml` | No |
| ETSI TS 119 182-1 (JAdES) | V1.2.1 (2024-07) | V1.2.1 is the latest published version | 2026-09-29 | `dpp-crypto::jades` | Unit tests; `jades-oracle.yml` (DSS) | No |
| ETSI TS 119 612 (Trusted Lists) | V2.3.1 | **V2.4.1 has since been published** | 2026-09-29 | `dpp-domain::trusted_list` | Unit tests | No |
| ETSI EN 319 102-1 (AdES validation) | None cited | V1.4.1 is the latest published version | 2026-09-29 | `dpp-domain` seal validation indications | Unit tests | No |
