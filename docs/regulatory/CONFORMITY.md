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
| CEN/CLC JTC 24 system standards | **Published 2026** — EN 18216–18246 (OJEU harmonisation citation pending) | Schema fields and API/authentication semantics tracked clause-by-clause against the published ENs |
| EU Battery Regulation 2023/1542 | In force | `BatteryData` struct implements Annex XIII fields (Art. 77 battery passport) |
| Textile DPP Delegated Act | Pending — an ESPR working-plan priority | `TextileData` struct held provisional; validated structurally until the act finalises |
| GS1 Digital Link v1.2 | Published | `DigitalLink` parser covers AI 01, 21, 10 |
| IDTA AAS Metamodel (IDTA-01001-3-0) | Published | `aas` module maps DPP to AAS Submodel |
| W3C VC Data Model v2.0 | CR | `DppAccessCredential` follows VC envelope structure |

## Three-Tier Access Model (ESPR Art. 10)

The ESPR mandates three levels of access to DPP data:

1. **Public** — No credential required. Includes fibre composition, country
   of manufacturing, care instructions, environmental metrics.

2. **Professional** — Requires a Verifiable Credential proving the holder's
   role (repairer, recycler, remanufacturer). Grants access to SVHC data,
   disassembly instructions, spare parts availability.

3. **Confidential** — Requires an institutional DID (market surveillance
   authority, customs, notified body). Grants access to compliance reports,
   audit history, supply chain traceability, JWS signatures.

### Implementation

- `dpp-crypto::credential` — W3C VC issuance and verification with
  role-based access tier mapping.
- `dpp-crypto::access_policy` — Stateless policy engine that filters JSON
  fields based on the caller's tier and a `SectorAccessPolicy`.
- Integration test: `tests/access_tier_gatekeeping.rs` validates all three
  tiers with realistic credentials.

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
- Integration test: `tests/transfer_of_responsibility.rs` covers full
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

The textile v1.1.0 schema covers the fields carried by the published
CEN/CLC JTC 24 system standards (EN 18216–18246, 2026) and their data-model
semantics:

- `fibreComposition` (with per-fibre `countryOfOrigin`)
- `countryOfManufacturing` (ISO 3166-1 alpha-2 enforced)
- `careInstructions`
- `chemicalComplianceStandard`

And all anticipated environmental and professional fields:

- `carbonFootprintKgCo2e`, `waterUseLitres`, `microplasticSheddingMgPerWash`
- `durabilityScore`, `repairScore`, `expectedWashCycles`
- `svhcSubstances` (CAS number, concentration, SCIP notification)
- `disassemblyInstructions`, `sparePartsAvailable`

Integration test: `tests/schema_conformity.rs` asserts field coverage.

## GS1 Interoperability

- **Digital Link** — Full AI 01/21/10 parsing and building, compliant with
  GS1 Digital Link URI Syntax v1.2.
- **Link-type Negotiation** — Content negotiation returning different DPP
  representations (JSON-LD, HTML, AAS) based on the `linkType` query parameter.
- **AAS Submodel Mapping** — Automatic conversion of DPP JSON to IDTA AAS
  SubmodelElement structures for Industry 4.0 / Catena-X interoperability.

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
| Textile end-to-end | `tests/textile_end_to_end.rs` | Passport lifecycle, AAS, GS1, credentials |
| Transfer of responsibility | `tests/transfer_of_responsibility.rs` | Transfer chain, provenance, error cases |
| Access tier gatekeeping | `tests/access_tier_gatekeeping.rs` | All three tiers, edge cases, custom policies |
| Schema conformity | `tests/schema_conformity.rs` | JTC 24 field coverage, structure validation |
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
