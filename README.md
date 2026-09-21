# Odal Node Core

**EU Digital Product Passport Standard Library**

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/odal-node/dpp-core/actions/workflows/ci.yml/badge.svg)](https://github.com/odal-node/dpp-core/actions/workflows/ci.yml)
[![Rust 1.96+](https://img.shields.io/badge/Rust-1.96%2B-orange.svg)](https://www.rust-lang.org/)
[![Status: Active Development](https://img.shields.io/badge/Status-Active%20Development-green.svg)]()

Pure, stateless Rust library for EU ESPR Digital Product Passport compliance. Domain types, cryptographic signing (Ed25519 + JWS), W3C Verifiable Credentials, GS1 Digital Link resolution, schema validation, and AAS submodel mapping. No database, no HTTP framework, no infrastructure dependencies.

Anyone building DPP tooling can use this library as the foundation. It is the standard, not the product.

> Note: This project is in active development.

> *"We provide the pipe, not the truth."* Odal Node uses a **proof-bound architecture**: product data is validated locally, signed with your key, and published as a verifiable passport. The transient import files are discarded after signing; the signed passport itself — the full product data, every field bound to a proof and gated by its disclosure class — is what the node retains and serves. What the world verifies is a cryptographic proof over real, tiered data, not a bare hash standing in for it.

---

## Why This Exists

EU law is switching on machine-readable Digital Product Passports product group by product group: battery passports become mandatory on **18 February 2027** (Reg. 2023/1542), the unsold-goods rules are in force **now** (ESPR Art. 24/25), detergents follow on 23 September 2029 (Reg. 2026/405), and the ESPR working plan queues textiles, steel and more behind them. Six European DPP system standards were cited in the Official Journal on **15 July 2026** by Commission Implementing Decision (EU) 2026/1736: EN 18216, 18219, 18220, 18221, 18222 and 18223. No affordable, developer-friendly infrastructure exists for the millions of SMEs who need to comply.

**Odal is that infrastructure**: sovereign, standards-compliant, self-hostable. No vendor lock-in, no black-box algorithms, no enterprise-tier licensing.

---

## The Compilation Test

```
cargo build --workspace
```

Succeeds with zero infrastructure running. No DB, no Redis, no env vars. If it needs infrastructure, it doesn't belong here.

---

## Crate Architecture

```
dpp-core/
  crates/
    dpp-domain .......... Domain types, port traits, VersionedSchemaRegistry, JSON Schema validation
      schemas/ .......... Versioned JSON Schemas for 12 product groups (battery, textile, electronics, …), embedded via include_str!
    dpp-crypto .......... Ed25519 keys, AES-256-GCM, JWS sign/verify, JAdES
    dpp-digital-link .... GS1 Digital Link parser and link-type negotiation
    dpp-aas ............. Asset Administration Shell (AAS) shells and submodels
    dpp-vc .............. W3C Verifiable Credentials, did:web, JSON-LD context
    dpp-plugin-traits ... Wasm product group plugin ABI (no_std compatible, capability negotiation)
    dpp-plugin-sdk ...... Guest-side plugin SDK: export_plugin! macro + Validator
    dpp-rules ........... Pure no_std cross-field regulatory rules, shared by dpp-domain and plugins
    dpp-registry ........ EU Central Registry interface types (wasm32-safe)
    dpp-calc ............ EU-methodology calculators (CO2e, repairability), pure functions
    dpp-vocab ........... External vocabulary authorities, one file per authority, with what we verified
    dpp-tests ........... Cross-crate integration tests and the structural tripwires
  plugins/ .............. 10 Wasm product group plugins (wasm32-wasip1, excluded from workspace)
```

---

## Regulatory Coverage

| Regulation | Status | dpp-core Implementation |
|---|---|---|
| **ESPR** (EU 2024/1781) | In force; unsold-goods rules (Art. 24/25) apply since Jul 2026 | Core data model (Art. 9-13, Annex III), access rights per Art. 11(b), unsold-goods product group, transfer-of-responsibility design (not a distinct ESPR article — see below) |
| **Battery Regulation** (EU 2023/1542) | In force — passport mandatory **18 Feb 2027** | `BatteryData` struct, Annex XIII fields, product group schema |
| **Textile DPP Delegated Act** | Pending (ESPR working-plan priority) | `TextileData` with SVHC disclosure, per-fibre traceability, durability metrics — provisional until the act finalises |
| **CEN/CLC JTC 24 system standards** | Six cited in the OJ on 15 Jul 2026 by CID (EU) 2026/1736: EN 18216, 18219, 18220, 18221, 18222, 18223 | **No conformance claimed.** No clause-by-clause assessment is published here. The ESPR Art. 41(2) presumption attaches to the cited standards, and claiming it requires an assessment we have not published — not merely a citation, which now exists |
| **GS1 Digital Link** | Published | AI 01/21/10 parsing, link-type negotiation. **No URI Syntax revision is claimed** — see `dpp-digital-link`'s module docs: an earlier *v1.2* claim had no recorded basis, the parser has not been diffed against 1.6.0:2022, and naming either would assert something unverified |
| **IDTA AAS Metamodel** | Published | Passport-to-AAS shell and submodel mapping. AAS-shaped output carrying **our own** semantics — every emitted `semanticId` is `urn:odal-node:*`; no IDTA conformance is claimed |
| **W3C VC Data Model v2.0** | Published | `DppAccessCredential` mapping operator roles to an `Audience` |

---

## Key Features

### Access Control — an Art. 77(2) lattice, not a ranking

Regulation (EU) 2023/1542 Art. 77(2) assigns three audiences to four Annex XIII
data sets. Crucially it is **not an ordering**: conformity test reports (point 3)
go to authorities only, and individual-battery use data (point 4) goes to
legitimate-interest holders only — so neither audience contains the other, and no
integer "tier" comparison can express the assignment.

| Audience | Annex XIII | Sees |
|---|---|---|
| **Public** | point 1 | Public battery-model information. No credential. |
| **Legitimate interest** | points 2 and 4 | Detailed composition, dismantling and safety, **plus** individual-item data: state of health, use history, status. Requires a VC proving the interest (repairer, remanufacturer, second-life operator, recycler). |
| **Authority** | points 2 and 3 | The same point-2 data, **plus** conformity test reports — but **not** point-4 individual data. Notified bodies, market surveillance, customs, the Commission. |

`Audience::may_see(Disclosure)` is the whole assignment in one table. ESPR itself
(Art. 9(2)(f), Art. 11(b)) requires per-actor access rights to be set by each
product group's delegated act rather than mandating a fixed set, so non-battery
product groups reuse the same vocabulary via each product group manifest's `disclosure` map.

### Transfer of Responsibility

When a product undergoes remanufacturing, repurposing, or preparation for reuse, DPP responsibility transfers to the new economic operator. ESPR has no distinct "transfer of responsibility" article by that name — this design follows from the general data-accuracy duty (Art. 9(1): DPP data "shall be accurate, complete and up to date") and the registry-upload duty (Art. 13(4)). The `TransferChain` provides:

- Append-only provenance log with state machine validation
- DID-identified economic operators with typed roles
- Dual-signature transfer records (JWS from both parties)
- Rejection of invalid transfers (wrong operator, duplicate pending)

### Evidence Dossiers

A self-contained, signed **evidence dossier** — passport, both JWS proofs, the issuer's DID document, the hash-chained audit trail, and the transfer chain in one canonical document — and its verification engine (independent checks: manifest signature, content integrity, both JWS, audit-chain linkage, transfer signatures) are a hosting-side feature, not part of this library: generating a dossier needs persistence and an audit trail, and checking one needs to fetch. What this crate contributes are the primitives that make either possible — `dpp-crypto`'s Ed25519/JWS, `dpp-vc`'s credentials and DID documents, and the domain types a dossier snapshots.

### Product Identity — EN 18219 clause 5, not GS1 only

A passport identifies its product under **one of three schemes**, which clause 5
offers as alternatives rather than a hierarchy:

| Scheme | Carries | Who can issue one |
|---|---|---|
| 1 — GS1 Digital Link | a 14-digit GTIN, check-digit validated | needs a GS1 Company Identification Number |
| 2 — Identification Link (EN IEC 61406) | an absolute `https` URL | self-issuing: a registered web domain is enough |
| 3 — DID (W3C DID v1.0) | a `did:web`, `did:ethr` or `did:ebsi` URI | self-issuing, same |

`ProductIdentifier` is the typed home for all three. Deserialisation is routed
through the constructors, so an identifier that could not have been built cannot
be read back either.

**Why this matters beyond tidiness:** requiring a GTIN is requiring GS1
membership. Schemes 2 and 3 exist so a manufacturer without one can still issue a
conformant passport, and modelling only scheme 1 quietly excludes them.

The identifier travels on `RegistrationRequest` rather than being re-derived from
the data carrier. 🚨 **A carrier is not an identifier** — a Digital Link happens
to contain a scheme 1 GTIN, a scheme 2 or 3 carrier contains no such thing, and
reading one out of the other is where an invented value gets submitted to a
public authority. `ProductIdentifier: TryFrom<&identifier::ProductIdentifier>` in
`dpp-registry` maps the three schemes and **refuses** an unmapped one rather than
inventing a scheme string.

### Schema Validation

Versioned JSON schemas at `crates/dpp-domain/schemas/{product-group}/v{version}.json` (embedded into the crate so they ship with it on publish):

| Product group | Versions | Key Fields |
|---|---|---|
| battery | v1.0.0, v2.0.0 – v2.7.0 | Chemistry, capacity, Art. 8 recycled content, Annex VII state of health and expected lifetime, placing-on-market date |
| electronics | v1.0.0 – v1.4.0 | Repairability, spare parts, substances of concern |
| textile | v1.0.0 – v1.3.0 | Fibre composition, SVHC, durability, microplastics |
| furniture | v1.0.0 – v1.3.0 | Product group-specific delegated-act fields |
| aluminium, construction, detergent, steel, toy | v1.0.0 – v1.2.0 each | Product group-specific delegated-act fields; steel adds CO2 intensity, scrap content, production method |
| mattress, tyre | v1.0.0 – v1.1.0 each | Product group-specific delegated-act fields |
| unsold-goods | v2.0.0 | Art. 25 destruction ban compliance |

The current version per group is the one the catalog serves
(`product-groups/*.json`, `currentSchemaVersion`) — read it there rather than
from this table, which is hand-maintained and has been behind before.

Twelve product groups. `unsold-goods` starts at v2.0.0: its v1 shape was
replaced outright when the disclosure was rebuilt to Impl. Reg. (EU) 2026/2
Annex I, so there is no v1 left to compare against.

The `VersionedSchemaRegistry` embeds schemas at compile time and supports runtime hot-reload for new versions. Read-time **upcast lenses** (`schemas::lens`) transform an old record's product group data to a newer schema version on read, so signed passports stay byte-identical yet remain consumable as delegated acts evolve the schema (upcast only).

### GS1 & Industry 4.0 Interoperability

- **Digital Link** — Full AI 01/21/10 parsing and building. No GS1 URI Syntax revision is claimed, deliberately: see the note in the Regulatory Coverage table
- **Link-type Negotiation** — Content negotiation returning JSON, JSON-LD, HTML, or AAS representations
- **AAS Submodel Mapping** — Passport-to-AAS shells and submodels for Industry 4.0 data spaces, carrying `urn:odal-node:*` semantics rather than a standards body's

### Wasm Product group Plugins

Compliance logic ships as sandboxed Wasm modules (`wasm32-wasip1`). Ten product group
plugins live under `plugins/` — battery (the reference implementation), textile,
electronics, steel, aluminium, construction, detergent, furniture, toy, and tyre.
Highlights:

| Plugin | Product groups | Key Rule |
|---|---|---|
| `product-group-battery.wasm` | battery | Battery Regulation 2023/1542 (reference implementation) |
| `product-group-textile.wasm` | textile, unsoldGoods | ESPR Art. 25 destruction ban (July 19, 2026) |
| `product-group-steel.wasm` | steel | CBAM CO2e/tonne thresholds |

Plugin ABI supports capability negotiation and semantic versioning with compatibility checking.

**Writing a plugin's `validate_input`.** `dpp-plugin-sdk`'s `Validator` is a
fluent collector that reports *every* failure rather than stopping at the first.
For the product identifier, use `require_product_identifier` — it picks which
field to check from the declared scheme, so a scheme 2 or 3 record is not asked
for a GTIN it cannot have:

```rust
Validator::new(input)
    .require_product_identifier("productIdentifier")
    .require_str("batteryChemistry")
    .require_positive("nominalVoltageV")
    .finish()
```

🚨 **`require_gtin` is not the field-level equivalent.** It reads a flat
top-level `gtin` key, which product group data no longer carries — the GTIN now
lives *inside* `productIdentifier`, and only under scheme 1. A plugin still
asking for the bare field reports *"gtin is required"* against a record that
identifies itself perfectly well. `require_gtin` remains correct for a genuinely
bare GS1 field, and is what `require_product_identifier` uses internally for the
scheme 1 branch.

An unrecognised scheme is **refused rather than skipped**: a scheme string nobody
has mapped is exactly where an invented identifier otherwise passes unexamined.

---

## Port Traits

The eight port traits define the core/platform boundary. Any downstream project implements these against its own infrastructure:

| Trait | Kind | Purpose |
|---|---|---|
| `PassportRepository` | async | CRUD for DPP records |
| `ComplianceRegistry` + `ComplianceStrategy` | sync | Product group-specific compliance dispatch |
| `IdentityPort` | async | Sign and verify passport JWS |
| `PluginHost` | sync | Wasm plugin dispatch |
| `BackupCopyPort` | async | The ESPR Art. 10(4) third-party back-up copy |
| `RegistrySyncPort` | async | EU Central Registry registration and status sync |
| `SealPort` | async | eIDAS qualified electronic seal (ESPR Art. 13 / eIDAS 910/2014) |

---

## Quick Start

```bash
git clone https://github.com/odal-node/dpp-core.git
cd dpp-core

cargo build --workspace          # zero infrastructure needed
cargo nextest run --workspace    # full unit + integration suite
just check                       # fmt + clippy + test + doctests + plugins + doc + lockfiles + audit
```

No Docker, no database, no env vars.

### Runnable Examples

```bash
cargo run -p dpp-domain --example create_passport                # Create & validate a textile DPP
cargo run -p dpp-crypto --example sign_and_verify                # Keystore, Ed25519 key, JWS sign
cargo run -p dpp-vc --example credential_and_transfer            # Issue a VC, transfer responsibility
cargo run -p dpp-digital-link --example parse_and_negotiate      # Parse GS1 links, negotiate a link type
cargo run -p dpp-aas --example passport_to_aas                   # Map a passport to an AAS shell
```

---


## Documentation

**Start with the guided index: [docs/README.md](docs/README.md)** — grouped by question, with a three-document reading path for newcomers.

| Document | Description |
|---|---|
| [BLUEPRINT.md](docs/project/BLUEPRINT.md) | Project vision, guiding principles, non-goals |
| [ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md) | Core library architecture and module design |
| [DATA-MODEL.md](docs/architecture/DATA-MODEL.md) | DPP canonical schema (ESPR / Battery Regulation aligned) |
| [IDENTITY.md](docs/architecture/IDENTITY.md) | `did:web` and Verifiable Credential deep dive |
| [PLUGIN-HOST.md](docs/architecture/PLUGIN-HOST.md) | Wasm plugin sandbox design and ABI contract |
| [DESIGN-PATTERNS.md](docs/architecture/DESIGN-PATTERNS.md) | Hexagonal architecture, open-core boundary patterns |
| [CONFORMITY.md](docs/regulatory/CONFORMITY.md) | Regulatory alignment statement for assessment bodies |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contributor guide: setup, conventions, PR workflow |
| [SECURITY.md](SECURITY.md) | Vulnerability disclosure policy |
| [GOVERNANCE.md](GOVERNANCE.md) | Decision-making structure and maintainer authority |
| [CHANGELOG.md](CHANGELOG.md) | Release history, one entry per version |


## License

[Apache License 2.0](LICENSE)

## Security

Do **not** open public issues for security vulnerabilities. Report privately to **security@odal-node.io** — see [SECURITY.md](SECURITY.md) for full disclosure policy.

---

*Built by [Odal Node](https://odal-node.io)*