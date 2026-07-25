# Conformity Statement

## Purpose

This document records the regulatory alignment of `dpp-core` with the EU
Ecodesign for Sustainable Products Regulation (ESPR, Regulation (EU)
2024/1781) and the anticipated sector delegated acts. It is intended for
conformity assessment bodies, GS1 Solution Partner reviewers, and pilot
programme evaluators.

## Regulatory References

| Reference | Status | dpp-core Alignment |
|---|---|---|
| ESPR (EU) 2024/1781 | In force | Core data model follows Art. 8–13 requirements |
| CEN/CLC JTC 24 system standards | Six published May 2026 (EN 18216/18219/18220/18221/18222/18223); EN 18239 + 18246 at FprEN, expected ~Sep 2026; OJEU harmonisation citation pending | Schema fields and API/authentication semantics tracked clause-by-clause against the published ENs |
| EU Battery Regulation 2023/1542 | In force | `BatteryData` struct implements Annex XIII fields (Art. 77 battery passport) |
| Textile DPP Delegated Act | Pending — an ESPR working-plan priority | `TextileData` struct held provisional; validated structurally until the act finalises |
| GS1 Digital Link v1.2 | Published | `DigitalLink` parser covers AI 01, 21, 10 |
| IDTA AAS Metamodel (IDTA-01001-3-0) | Published | `aas` module maps DPP to AAS Submodel |
| W3C VC Data Model v2.0 | CR | `DppAccessCredential` follows VC envelope structure |

## Access Model — an Art. 77(2) lattice, not a ranking

Regulation (EU) 2023/1542 Art. 77(2) assigns three **audiences** to the four
Annex XIII data sets. Read against the verbatim OJ text, the assignment is not
an ordering:

| Audience | Annex XIII points | Content |
|---|---|---|
| (a) General public | 1 | Public model-level information. No credential. |
| (b) Notified bodies, market surveillance authorities, the Commission | 2 and 3 | Composition, dismantling and safety, **plus** conformity test reports. |
| (c) Persons with a legitimate interest | 2 and 4 | The same point-2 data, **plus** individual-item data: state of health, use history, status. |

Point 3 is authority-only and point 4 is legitimate-interest-only, so **neither
audience contains the other**. No integer "tier" comparison can express this: any
`>=` test necessarily either discloses to authorities the individual-item data
Art. 77(2)(b) withholds, or hides point-2 data from someone entitled to it. The
implementation therefore models two independent types — `Audience` (who is
asking) and `Disclosure` (how restricted a field is) — related by a single
total function, `Audience::may_see(Disclosure)`.

ESPR does not itself fix a set of access levels. Read against the verbatim OJ
text of Regulation (EU) 2024/1781, three provisions carry the point:

- **Art. 9(2)(f)** — the delegated act specifies "the actors that are to have
  access to data in the digital product passport and to what data they are to
  have access".
- **Art. 10(1)(g)** — access "shall be regulated in accordance with the
  essential requirements set out in this Article and Article 11 and with the
  specific access rights at product group level as specified in the applicable
  delegated act".
- **Art. 11(b)** — the listed actors "shall have free of charge and easy access
  to the digital product passport based on their respective access rights set
  out in the applicable delegated act".

So the access lattice is set per product group, not by ESPR itself. Non-battery
sectors therefore reuse this same vocabulary through each sector manifest's
`disclosure` map rather than inheriting a hardcoded ladder.

> **Superseded.** Releases up to and including 0.10.0 implemented an ordered
> three-tier model (`AccessTier::{Public, Professional, Confidential}`). It was
> removed in 0.11.0 for the reason above. Assessments performed against the
> earlier model should be re-read against this section.

### Implementation

- `dpp_domain::domain::identity` — `Audience`, `Disclosure`, and
  `PASSPORT_FIELD_DISCLOSURE`, the single source for the disclosure class of
  every non-public top-level passport field.
- `dpp_crypto::access::credential` — W3C VC issuance and verification, mapping
  an operator role to an `Audience`.
- `dpp_crypto::access::policy` / `access::filter` — stateless policy engine that
  filters JSON fields against the caller's `Audience` and a
  `SectorAccessPolicy`.
- Integration test: `crates/dpp-tests/tests/access_gatekeeping.rs` exercises all
  three audiences with realistic credentials.

## Transfer of Responsibility

No distinct "transfer of responsibility" article exists in ESPR by that name (checked against the
verbatim OJ text of Regulation (EU) 2024/1781); this design follows from the general data-accuracy
duty (Art. 9(1)) and the registry-upload duty (Art. 13(4)), not a single dedicated article. The
prior "Art. 12" citation was wrong — Art. 12 is "Unique identifiers" (operator/facility identifier
issuance mechanics).

When a product undergoes remanufacturing, repurposing, or preparation for
reuse, the new economic operator assumes full DPP responsibility. The
`dpp-domain::transfer` module implements:

- `TransferChain` — Append-only provenance log with state machine validation.
- `ResponsibleOperator` — DID-identified economic operator with role typing.
- `TransferRecord` — Dual-signature transfer event (JWS from both parties).
- Integration test: `crates/dpp-tests/tests/transfer_of_responsibility.rs` covers full
  lifecycle, error cases, and serialisation round-trips.

## Schema Validation

### Versioned Schemas

All sector schemas reside in `schemas/{sector}/v{version}.json` and follow
JSON Schema Draft-07. The `VersionedSchemaRegistry` embeds them at compile
time via `include_str!()`.

| Sector | Versions | Fields Covered |
|---|---|---|
| textile | v1.0.0, v1.1.0 | Fibre composition, SVHC, durability, microplastics |
| battery | v1.0.0 | Chemistry, capacity, recycled content, SoH |
| steel | v1.0.0 | CO₂ intensity, scrap content, production method |
| unsold-goods | v1.0.0 | Unsold goods destruction reporting |

### JTC 24 Field Coverage

The textile v1.2.0 schema covers the fields carried by the
CEN/CLC JTC 24 system standards (the six ENs published May 2026 —
EN 18216/18219/18220/18221/18222/18223) and their data-model
semantics:

- `fibreComposition` (with per-fibre `countryOfOrigin`)
- `countryOfOrigin` (ISO 3166-1 alpha-2 enforced)
- `careInstructions`
- `chemicalComplianceStandard`

And all anticipated environmental and professional fields:

- `carbonFootprintKgCo2e`, `waterUseLitres`, `microplasticSheddingMgPerWash`
- `durabilityScore`, `repairScore`, `expectedWashCycles`
- `svhcSubstances` (CAS number, concentration, SCIP notification)
- `disassemblyInstructions`, `sparePartsAvailable`

Integration test: `crates/dpp-tests/tests/schema_conformity.rs` asserts field coverage.

## GS1 Interoperability

- **Digital Link** — Full AI 01/21/10 parsing and building, compliant with
  GS1 Digital Link URI Syntax v1.2.
- **Link-type Negotiation** — Content negotiation returning different DPP
  representations (JSON-LD, HTML, AAS) based on the `linkType` query parameter.
- **AAS Submodel Mapping** — Automatic conversion of DPP JSON to IDTA AAS
  SubmodelElement structures for Industry 4.0 / Catena-X interoperability.

## Unique Identifier — ISO/IEC 15459 (Battery Reg. Art. 77(3))

Art. 77(3) of Regulation (EU) 2023/1542 requires that *"the QR code and the
unique identifier shall comply with ISO/IEC standards 15459-1:2014,
15459-2:2015, 15459-3:2014, 15459-4:2014, 15459-5:2014 and 15459-6:2014"*.

**Position.** The carrier is a GS1 Digital Link URI over a GS1 identification
key: GTIN (AI 01) plus an item serial (AI 21), optionally a batch/lot (AI 10) —
i.e. a serialised GTIN. 🔶 GS1 is a registered Issuing Agency under ISO/IEC
15459, so identifiers issued under GS1 keys carry a registered Issuing Agency
Code and inherit the scheme's global-uniqueness guarantees. Conformance is
therefore claimed **through GS1**, not by independent implementation of the
ISO parts.

**What is verified.** The AI 21 serial is exactly 20 characters from `[0-9a-f]`,
within the GS1 General Specifications limit that the `DigitalLink` parser
enforces; the URI syntax is GS1 Digital Link v1.2. Both are covered by tests.

**What is not.** 🔶 The ISO/IEC 15459 parts are paywalled and have **not** been
read against primary text. The claim above rests on GS1's registration as an
Issuing Agency and on secondary sources, not on the standard's own wording. Do
not restate it as a verified conformance assertion, and do not put it in
customer-facing material, until someone has read the parts — in particular
15459-3 (common rules) and 15459-4 (individual products), which are the two that
bear on a per-item product identifier.

**Serial construction.** The AI 21 serial is derived from the passport UUIDv7's
last ten bytes (`rand_a` + `rand_b`, 74 random bits), not its first ten. The
leading six bytes of a UUIDv7 are a millisecond timestamp: deriving the serial
from them produced a monotonically increasing serial whose first twelve hex
characters decoded to the passport's creation instant, so a QR code on a
physical battery disclosed when it was created and, across several codes, the
production order and rate. Fixed; regression tests cover both properties.

**Open question.** ISO/IEC 15459 requires uniqueness to be *persistent over
time*. The serial is deterministic from the passport UUID and unique per
passport, but nothing currently prevents two passports being issued for one
physical item, or a reissued passport receiving a different serial for the same
battery. That is an operational guarantee the engine must make, not one this
crate can enforce.

## Processor Limits — Art. 78(d)

Art. 78(d) of Regulation (EU) 2023/1542 forbids an operator authorised to act on
behalf of the responsible economic operator from selling, re-using or processing
passport data *"beyond what is necessary for the provision of the relevant
storing or processing services"*.

**What satisfies it is architectural, not a policy promise.** Every deployment is
single-operator — one node per operator, self-hosted or hosted, with no shared
cluster — so no surface exists on which one customer's passport data and
another's can be seen together. A cross-customer benchmark is not something the
system declines to build; it is something it has no place to compute.

**Where the constraint lives in code.** `PassportRepository` is the primary
persistence surface — resolver scan telemetry is processed data too — and its
`list` and `count` methods are the only ones that see more than one passport. The port documents the Art. 78(d) limit on
implementors directly, so a future backing store cannot acquire an analytics
sideline without someone editing past the constraint.

**What it does not restrict.** An operator analysing its own passports. The
prohibition binds the processor acting on the operator's behalf, not the
operator.

**Already applied.** Resolver scan telemetry records only per-passport, per-day,
per-variant counts — no IP address, user agent or session identifier, because the
schema has no column for one. That design predates this section; Art. 78(d) is
the article it answers to.

**Residual, engine-side.** `dpp-core` is stateless and holds no data, so it can
only state the constraint and place it at the seam. Enforcement — retention of
logs, backup handling, what a hosted control plane may read — is a `dpp-engine`
and infrastructure concern and is not evidenced here.

## Cryptographic Foundations

- **Ed25519** — All signing operations use Ed25519 (EdDSA) as specified by
  the ESPR implementing guidance.
- **AES-256-GCM** — Key encryption at rest.
- **did:web** — DID method for operator identification, with DID Document
  builder following W3C DID Core v1.0.
- **JWS (RFC 7515)** — Compact serialisation for passport and transfer signatures.

## Wasm Plugin Architecture

Sector-specific compliance logic runs as sandboxed Wasm modules
(`wasm32-wasip1`) loaded by the platform. The plugin ABI includes:

- Capability negotiation (plugins declare supported operations).
- Semantic versioning with compatibility checking.
- Stateless invocation (no shared memory across plugin calls).

## Test Coverage

| Test Suite | Location | Coverage |
|---|---|---|
| Textile end-to-end | `crates/dpp-tests/tests/textile_end_to_end.rs` | Passport lifecycle, AAS, GS1, credentials |
| Transfer of responsibility | `crates/dpp-tests/tests/transfer_of_responsibility.rs` | Transfer chain, provenance, error cases |
| Audience gatekeeping | `crates/dpp-tests/tests/access_gatekeeping.rs` | All three audiences, edge cases, custom policies |
| Schema conformity | `crates/dpp-tests/tests/schema_conformity.rs` | JTC 24 field coverage, structure validation |
| Unit tests | Per-module `#[cfg(test)]` | All crates have inline unit tests |

## CI/CD Gate

The `just check` recipe and GitHub Actions CI run:

1. `cargo fmt --all --check` — Formatting consistency.
2. `cargo clippy --workspace --all-targets -- -D warnings` — Zero warnings.
3. `cargo nextest run --workspace` — All unit and integration tests.
4. `cargo audit` — RustSec advisory database check.

## Known Gaps

1. **JWS signature verification** in `jws_verifier` performs structural checks
   but does not yet resolve DIDs from the network to fetch public keys. This
   requires the platform's HTTP client (not available in the pure core).

2. **StatusList2021** revocation checking is modelled but not implemented
   (requires HTTP fetching of the status list credential).

3. **Schema hot-reload** is implemented but the file-watching trigger lives
   in the platform crate.

4. **Wasm plugins** are excluded from workspace CI. The `wasm-build.yml`
   workflow handles them separately.

## Contact

For conformity assessment inquiries: dev@odal-node.io
