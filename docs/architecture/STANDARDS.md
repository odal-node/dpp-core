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
| RFC 8032 | Edwards-Curve Digital Signature Algorithm (EdDSA) | Informational | 2026-09-29 | `dpp-crypto` JWS signing and strict verification | Delegated to `ed25519-dalek`; cross-library test | No |
| RFC 8037 | CFRG ECDH and Signatures in JOSE | Proposed Standard; **updated by RFC 9864**, which deprecates the JOSE `alg` value `EdDSA`. `dpp-crypto` still writes it, for the reason in the RFC 9864 row | 2026-10-05 | `dpp-crypto` `alg`/`crv` names; `dpp-vc` JWK `kty` | Unit tests | No |
| RFC 8410 | Algorithm Identifiers for Ed25519, Ed448, X25519, and X448 for Use in the Internet X.509 PKI | Proposed Standard; updated by RFC 9295 | 2026-09-29 | `dpp-crypto` JAdES oracle artefact (PKCS#8 key) | Artefact validated in `jades-oracle.yml` | No |
| RFC 8725 | JSON Web Token Best Current Practices | Best Current Practice; updates RFC 7519 | 2026-10-05 | `dpp-vc` VC-JWT verification requires `typ: vc+jwt`, the explicit typing of §3.11 | Unit tests | No |
| RFC 8785 | JSON Canonicalization Scheme (JCS) | Informational | 2026-09-29 | Signing and content-binding canonical form across `dpp-crypto`, `dpp-domain`, `dpp-rules`, `dpp-calc`, `dpp-vc` | Delegated to `serde_jcs`; unit tests | No |
| RFC 9106 | Argon2 Memory-Hard Function for Password Hashing and Proof-of-Work Applications | Informational | 2026-09-29 | `dpp-crypto` keystore key derivation | Delegated to `argon2`; frozen vectors produced by `argon2` 0.5.3, **not** the RFC's own vectors | No |
| RFC 9110 | HTTP Semantics | Internet Standard | 2026-09-29 | `dpp-digital-link` `Accept` q-value parsing | Unit tests | No |
| RFC 9278 | JWK Thumbprint URI | Proposed Standard | 2026-10-05 | `dpp-crypto` thumbprint URI; `dpp-vc` verification-method fragments, JWK `kid`, and the SD-JWT VC `kid` | Unit tests | No |
| RFC 9562 | Universally Unique IDentifiers (UUIDs) | Proposed Standard | 2026-09-29 | `dpp-domain` passport id, UUIDv7 layout | Delegated to `uuid`; serial-derivation regression tests | No |
| RFC 9864 | Fully-Specified Algorithms for JSON Object Signing and Encryption (JOSE) and CBOR Object Signing and Encryption (COSE) | Proposed Standard; updates RFC 7518, RFC 8037, RFC 9053 | 2026-10-05 | `dpp-crypto` accepts `alg: Ed25519` when verifying and still writes the deprecated `EdDSA`, under the exception in §4.4 for documented operational requirements: the European Commission's DSS 6.5 maps only `EdDSA` for JOSE, and the W3C VC-JOSE-COSE test suite signs its fixtures with it. Re-read both at each release; the emitted name changes when both have moved | Unit tests | No |
| RFC 9901 | Selective Disclosure for JSON Web Tokens | Proposed Standard | 2026-09-29 | `dpp-crypto::sd_jwt`; `dpp-vc::sd_jwt_vc` | **The RFC's own test vectors** (`rfc9901_vector_tests.rs`) | No |
| `draft-ietf-oauth-sd-jwt-vc-19` | SD-JWT-based Verifiable Digital Credentials (SD-JWT VC) | Internet-Draft; revision 19 is current, intended as a Proposed Standard, not yet an RFC | 2026-09-29 | `dpp-vc::sd_jwt_vc` | Unit tests | No |

## Other specifications

| Specification | Revision cited | Status as read | Read | Implemented in | Evidence | Conformance claimed |
|---|---|---|---|---|---|---|
| W3C Verifiable Credentials Data Model | v2.0 | W3C Recommendation, 15 May 2025 | 2026-09-29 | `dpp-vc::credential` (`DppAccessCredential`) | Unit tests; `access_gatekeeping.rs` | No |
| W3C Decentralized Identifiers (DIDs) | v1.0 | W3C Recommendation, 19 July 2022. v1.1 is a Candidate Recommendation Snapshot (5 March 2026) that defines its key properties by reference to Controlled Identifiers v1.0 | 2026-10-07 | `dpp-vc` DID document builder; DID syntax in `dpp-rules::common::identifier` | Unit tests; the W3C's own DID test suite, run over documents the builder produced, in `did-oracle.yml` | Yes. **Class:** a conforming DID document (DID Core 1.4). **Self-declared.** **Scope:** the documents `build_did_document` returns, which carry `@context`, `id`, `verificationMethod`, `authentication` and `assertionMethod`, in the JSON-LD representation (application/did+ld+json). The suite's `did-core-properties` and `did-production` suites check them against sections 5, 6.1, 6.3 and 7.3. **Not covered:** the plain JSON representation, which this library does not produce; section 4, which the suite does not test; DID syntax, resolution and consumption as claims of their own; and every rule about `service`, `controller`, `alsoKnownAs`, `keyAgreement`, `capabilityInvocation` and `capabilityDelegation`, since the documents carry none of them. The section 6 checks compare a document with the data model derived from it, so they carry little weight. **Known deviations:** none known. **Evidence:** `.github/workflows/did-oracle.yml`, `.github/scripts/did_suite_oracle.mjs`, `crates/dpp-vc/tests/did_suite_input.rs` |
| W3C Controlled Identifiers | v1.0 | W3C Recommendation, 15 May 2025 | 2026-10-05 | `dpp-vc` DID document: `JsonWebKey` verification methods, `cid/v1` context; `dpp-crypto` key resolution (the binding checks of §3.3) | Unit tests | No |
| W3C Securing Verifiable Credentials using JOSE and COSE | v1.0 | W3C Recommendation, 15 May 2025 | 2026-10-05 | `dpp-vc` VC-JWT `kid` (key discovery), `typ: vc+jwt` and `cty: vc` | Unit tests | No |
| `did:web` Method Specification | Unversioned | A W3C Credentials Community Group document whose own `specStatus` is `unofficial`. **Not a W3C standard** | 2026-09-29 | `dpp-vc` DID document builder | Unit tests | No |
| W3C Bitstring Status List | v1.0 | W3C Recommendation, 15 May 2025 | 2026-10-05 | `dpp-vc::status_list`; `credential::check_revocation`, which answers only an entry whose `statusPurpose` is `revocation` | Unit tests | No |
| W3C JSON-LD | 1.1 | W3C Recommendation, 16 July 2020. W3C's page shows no later version | 2026-10-07 | `dpp-vc::jsonld` context, the two credential contexts, and the DID document context | PyLD 3.3.0, an independent JSON-LD 1.1 processor, expands every context and every emitted document in `jsonld-oracle.yml`; unit tests; `jsonld_context.rs` | Yes. **Class:** a conforming JSON-LD 1.1 document, as judged by a JSON-LD 1.1 processor in its 1.1 processing mode. **Scope:** five contexts and seventeen documents, each built by the real builder: a DID document at a first key and after a rotation, the passport credential, the access credential with and without a status entry, and the framed passport of each of the twelve product groups. The W3C contexts they reference are served from vendored, hash-checked copies, and the processor is given no other and no base IRI, so every node identifier must be an absolute IRI or a blank node: a framed passport is named `urn:uuid:` and its id, the subject of its credential. Each of our contexts must also still expand after the W3C credentials v2 and DID v1 contexts, which protect `id`. **Known deviations:** a framed passport's context defines a term for each key of the envelope and for the product identifier, and for nothing beneath them, so a processor drops the keys nested below an envelope key, including every key of `productGroupData` except the identifier. The oracle lists that gap, fails on any other dropped property, and fails when the gap closes. **Not claimed:** what any IRI means, compaction, flattening, framing and RDF conversion as claims of their own, or JSON-LD 1.0. **Self-declared:** no body certifies a JSON-LD producer. **Evidence:** `.github/workflows/jsonld-oracle.yml`, `.github/scripts/jsonld_oracle.py`, `crates/dpp-tests/tests/jsonld_oracle_corpus.rs` and `crates/dpp-tests/fixtures/jsonld/` |
| JSON Schema | Draft-07 | Later drafts published (2019-09, 2020-12). The product group schemas pin Draft-07 | 2026-09-29 | `crates/dpp-domain/schemas/`, enforced by the `jsonschema` crate through `validator_for` | The official Draft-07 meta-schema, and the official test-suite files for the keywords and formats the schemas use, both vendored verbatim under `crates/dpp-tests/fixtures/json-schema/` and run by `json_schema_draft07.rs` | Yes. **Class:** the specification names none, so this claims the two checks it makes testable: a schema is valid against the Draft-07 meta-schema, and a validator returns the specified verdict for each keyword. **Scope:** every embedded product group schema of every version, and the pinned `jsonschema` crate as the registry builds it, with formats asserted and no remote retrieval, for the keywords and formats those schemas use. **Known deviations:** none among the cases run. Any other keyword is outside the claim, and a schema that starts using one fails `crates/dpp-tests/tests/json_schema_draft07.rs`. **Self-declared:** nobody certifies a JSON Schema implementation. **Evidence:** `crates/dpp-tests/fixtures/json-schema/` |
| GS1 Digital Link URI Syntax | **Not established** | EN 18219:2026 clause 6.3.2 names 1.6.0:2022. The parser has been diffed against no revision. See [`regulatory/CONFORMITY.md`](../regulatory/CONFORMITY.md) | 2026-09-29 | `dpp-digital-link` | Unit tests; CSET 82 membership judged by GS1's Barcode Syntax Engine in `gs1-oracle.yml` | No |
| IDTA Asset Administration Shell Part 1: Metamodel | IDTA-01001-3-0 | **IDTA-01001-3-2** (release 26-01, published 2026-07-21) is the current release, after 3-1 (release 25-01). 3-2 adds the `Batch` value to `AssetKind`, `createdAt` and `updatedAt` to `AdministrationInformation`, and the constraints AASd-137 and AASd-138 | 2026-10-07 | `dpp-aas`, carrying this library's own `urn:odal-node:*` semantics | Two independent implementations run over the committed Environments in `aas-oracle.yml`: `aas-core3.0` 1.1.4 from aas-core-works, and `aas-test-engines` 1.0.3, IDTA's own test tooling; the output is also validated against IDTA's JSON Schemas for 3.0, 3.1 and 3.2; unit tests | Yes. **Class:** an AAS Environment in the JSON serialisation that passes the metamodel's own meta-model check and satisfies its `AASd-xxx` constraints. **Scope:** IDTA-01001-3-0 only, for the public-audience Environment of each of the twelve product groups plus one scenario Environment, thirteen files, built from fully populated test passports. **Why 3-0 and not 3-2:** neither tool implements a later revision, because aas-test-engines 1.0.3 lists 3.0 alone and aas-core-works publishes no 3.2 package, so 3-2's constraints cannot be checked independently. **Known deviations:** `AASd-021` and `AASd-077` are not implemented by aas-test-engines and are checked by aas-core3.0 alone. The 3-2 additions are unchecked: AASd-137 and AASd-138, and the `Batch` asset kind, which this library does not emit (a batch is emitted as `Instance`). 3.1 and 3.2 are covered for JSON shape only. **Not claimed:** that any submodel matches a published submodel template, what any `semanticId` means (all are `urn:odal-node:*` by design), the AASX package format, the XML serialisation, or the Part 2 API. **Self-declared:** IDTA does not certify an AAS file. **Evidence:** `.github/workflows/aas-oracle.yml`, `.github/scripts/aas_test_engines_oracle.py`, `.github/scripts/aas_loader_oracle.py` and `crates/dpp-tests/fixtures/aas/environments/` |
| ETSI TS 119 182-1 (JAdES) | V1.2.1 (2024-07) | V1.2.1 is the latest published version. Its Annex B, which is normative, names the JSON Schemas this repository checks the protected header against | 2026-10-06 | `dpp-crypto::jades` | ETSI's own Annex B JSON Schema for the protected header, vendored under `crates/dpp-tests/fixtures/jades/` and run over every header shape the builder emits; DSS 6.2, the European Commission's reference implementation, run over a produced signature in `jades-oracle.yml`; unit tests | Yes. **Class:** the JAdES-B-B baseline level, as clause 6 and Table 1 set it out. **Scope:** structure only. The compact serialisation with the payload attached, so no `sigD`, at B-B and no higher, with `EdDSA`, the algorithm the evidence uses. What is checked is the protected header, with the signing certificate referenced by `x5t#S256` alone, or by `x5c` and `x5t#S256` together, which is the form the EU profile of Commission Implementing Regulation (EU) 2026/248, Annex I, asks for. **Known deviations:** a header with `x5c` alone, which the builder can still produce, satisfies ETSI's schema but DSS does not call it baseline, so the claim does not cover it. ETSI's schema declares `contentEncoding: base64` for the base64url `x5t#S256`, and is read here as either alphabet. DSS judges only the `x5c` with `x5t#S256` form. **Not claimed:** that a signature validates, is trusted, or is qualified. **Self-declared:** no body certifies a JAdES implementation. **Evidence:** `crates/dpp-tests/tests/jades_annex_b_schemas.rs`, `crates/dpp-tests/fixtures/jades/` and `.github/workflows/jades-oracle.yml` |
| ETSI TS 119 612 (Trusted Lists) | V2.3.1 and V2.4.1 | V2.4.1 is the latest published version. Implementing Regulations (EU) 2025/1945 and 2025/1946 name V2.3.1, and the trusted-list template of Implementing Decision (EU) 2015/1505, as amended by Implementing Decision (EU) 2025/2164, names V2.4.1. The two are the same in everything `trusted_list` reads | 2026-10-06 | `dpp-domain::trusted_list` | Unit tests; the two versions compared word for word | No. The standard defines the list a scheme operator publishes, and this crate consumes lists, so there is no conformance class for it to claim |
| ETSI EN 319 102-1 (AdES validation) | None cited | V1.4.1 is the latest published version | 2026-09-29 | `dpp-domain` seal validation indications | Unit tests | No |
