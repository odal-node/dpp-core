# Standards Register

One row per external technical specification this repository cites. Each row
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
- **Other rows are not machine-checked.** W3C, GS1, IDTA and ETSI identifiers
  take too many shapes to match reliably, so the re-read below is their only
  check.
- **Every status is re-read at release.** The Pre-Release Checklist in
  [`governance/RELEASE.md`](../governance/RELEASE.md) re-reads each row at its
  source and updates the date. The gate proves that every citation has a status
  somebody read and dated. It cannot prove that status is still current.

**"Conformance claimed" is `No` throughout.** Code here *implements* the parts of
a specification it uses. Claiming conformance would mean stating a profile and
the options chosen, and nothing here does that yet.

## IETF

| Spec | Title | Status as read | Read | Implemented in | Evidence | Conformance claimed |
|---|---|---|---|---|---|---|
| RFC 3339 | Date and Time on the Internet: Timestamps | Proposed Standard; updated by RFC 9557 | 2026-09-29 | `dpp-vc` snapshot validity instants | Unit tests | No |
| RFC 3986 | Uniform Resource Identifier (URI): Generic Syntax | Internet Standard; updated by RFC 7320, RFC 8820 | 2026-09-29 | `dpp-digital-link` path encoding; `dpp-aas` asset URIs; `dpp-vc` `did:web` paths | Unit tests | No |
| RFC 4151 | The 'tag' URI Scheme | Informational | 2026-09-29 | `dpp-vc` SD-JWT VC `vct` | Unit tests | No |
| RFC 7515 | JSON Web Signature (JWS) | Proposed Standard | 2026-09-29 | `dpp-crypto` JWS and JAdES headers; `dpp-domain` seal envelope | Unit tests; JAdES output checked by the European Commission's DSS in `jades-oracle.yml` | No |
| RFC 7517 | JSON Web Key (JWK) | Proposed Standard | 2026-09-29 | `dpp-vc` SD-JWT VC issuer metadata | Unit tests | No |
| RFC 7519 | JSON Web Token (JWT) | Proposed Standard; updated by RFC 7797, RFC 8725 | 2026-09-29 | `dpp-vc` SD-JWT VC validity claims | Unit tests | No |
| RFC 8032 | Edwards-Curve Digital Signature Algorithm (EdDSA) | Informational | 2026-09-29 | `dpp-crypto` JWS signing and strict verification | Delegated to `ed25519-dalek`; cross-library test | No |
| RFC 8037 | CFRG ECDH and Signatures in JOSE | Proposed Standard; **updated by RFC 9864**, which deprecates the JOSE `alg` value `EdDSA` that `dpp-crypto` emits. Kept pending the decision in #370 | 2026-09-29 | `dpp-crypto` `alg`/`crv` names; `dpp-vc` JWK `kty` | Unit tests | No |
| RFC 8410 | Algorithm Identifiers for Ed25519, Ed448, X25519, and X448 for Use in the Internet X.509 PKI | Proposed Standard; updated by RFC 9295 | 2026-09-29 | `dpp-crypto` JAdES oracle artefact (PKCS#8 key) | Artefact validated in `jades-oracle.yml` | No |
| RFC 8785 | JSON Canonicalization Scheme (JCS) | Informational | 2026-09-29 | Signing and content-binding canonical form across `dpp-crypto`, `dpp-domain`, `dpp-rules`, `dpp-calc`, `dpp-vc` | Delegated to `serde_jcs`; unit tests | No |
| RFC 9106 | Argon2 Memory-Hard Function for Password Hashing and Proof-of-Work Applications | Informational | 2026-09-29 | `dpp-crypto` keystore key derivation | Delegated to `argon2`; frozen vectors produced by `argon2` 0.5.3, **not** the RFC's own vectors | No |
| RFC 9110 | HTTP Semantics | Internet Standard | 2026-09-29 | `dpp-digital-link` `Accept` q-value parsing | Unit tests | No |
| RFC 9562 | Universally Unique IDentifiers (UUIDs) | Proposed Standard | 2026-09-29 | `dpp-domain` passport id, UUIDv7 layout | Delegated to `uuid`; serial-derivation regression tests | No |
| RFC 9901 | Selective Disclosure for JSON Web Tokens | Proposed Standard | 2026-09-29 | `dpp-crypto::sd_jwt`; `dpp-vc::sd_jwt_vc` | **The RFC's own test vectors** (`rfc9901_vector_tests.rs`) | No |
| `draft-ietf-oauth-sd-jwt-vc-19` | SD-JWT-based Verifiable Digital Credentials (SD-JWT VC) | Internet-Draft; revision 19 is current, intended as a Proposed Standard, not yet an RFC | 2026-09-29 | `dpp-vc::sd_jwt_vc` | Unit tests | No |

## Other specifications

| Specification | Revision cited | Status as read | Read | Implemented in | Evidence | Conformance claimed |
|---|---|---|---|---|---|---|
| W3C Verifiable Credentials Data Model | v2.0 | W3C Recommendation, 15 May 2025 | 2026-09-29 | `dpp-vc::credential` (`DppAccessCredential`) | Unit tests; `access_gatekeeping.rs` | No |
| W3C Decentralized Identifiers (DIDs) | v1.0 | W3C Recommendation, 19 July 2022 | 2026-09-29 | `dpp-vc` DID document builder; DID syntax in `dpp-rules::common::identifier` | Unit tests | No |
| `did:web` Method Specification | Unversioned | A W3C Credentials Community Group document whose own `specStatus` is `unofficial`. **Not a W3C standard** | 2026-09-29 | `dpp-vc` DID document builder | Unit tests | No |
| W3C Bitstring Status List | v1.0 | W3C Recommendation, 15 May 2025 | 2026-09-29 | `dpp-vc::status_list`; `credential::check_revocation` | Unit tests | No |
| W3C JSON-LD | 1.1 | W3C Recommendation, 16 July 2020 | 2026-09-29 | `dpp-vc::jsonld` context | Unit tests; `jsonld_context.rs` | No |
| JSON Schema | Draft-07 | Later drafts published (2019-09, 2020-12). The product group schemas pin Draft-07 | 2026-09-29 | `crates/dpp-domain/schemas/` | Every schema compiled and exercised by the schema tests | No |
| GS1 Digital Link URI Syntax | **Not established** | EN 18219:2026 clause 6.3.2 names 1.6.0:2022. The parser has been diffed against no revision. See [`regulatory/CONFORMITY.md`](../regulatory/CONFORMITY.md) | 2026-09-29 | `dpp-digital-link` | Unit tests; CSET 82 membership judged by GS1's Barcode Syntax Engine in `gs1-oracle.yml` | No |
| IDTA Asset Administration Shell Part 1: Metamodel | IDTA-01001-3-0 | IDTA's specification page lists **IDTA-01001-3-2** as the current release | 2026-09-29 | `dpp-aas`, carrying this library's own `urn:odal-node:*` semantics | Unit tests; environments run through an external AAS implementation in `aas-oracle.yml` | No |
| ETSI TS 119 182-1 (JAdES) | V1.2.1 (2024-07) | V1.2.1 is the latest published version | 2026-09-29 | `dpp-crypto::jades` | Unit tests; `jades-oracle.yml` (DSS) | No |
| ETSI TS 119 612 (Trusted Lists) | V2.3.1 | **V2.4.1 has since been published** | 2026-09-29 | `dpp-domain::trusted_list` | Unit tests | No |
| ETSI EN 319 102-1 (AdES validation) | None cited | V1.4.1 is the latest published version | 2026-09-29 | `dpp-domain` seal validation indications | Unit tests | No |
