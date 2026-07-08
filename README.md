# Odal Node Core

**EU Digital Product Passport Standard Library**

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/odal-node/dpp-core/actions/workflows/ci.yml/badge.svg)](https://github.com/odal-node/dpp-core/actions/workflows/ci.yml)
[![Rust 1.96+](https://img.shields.io/badge/Rust-1.96%2B-orange.svg)](https://www.rust-lang.org/)
[![Status: Active Development](https://img.shields.io/badge/Status-Active%20Development-green.svg)]()

Pure, stateless Rust library for EU ESPR Digital Product Passport compliance. Domain types, cryptographic signing (Ed25519 + JWS), W3C Verifiable Credentials, GS1 Digital Link resolution, schema validation, and AAS submodel mapping. No database, no HTTP framework, no infrastructure dependencies.

Anyone building DPP tooling can use this library as the foundation. It is the standard, not the product.

> Note: This project is in active development.

> *"We provide the pipe, not the truth."* Odal Node uses a **proof-bound architecture**: product data is validated locally, signed with your key, and published as a verifiable passport — the raw inputs are discarded after signing, never retained by the node. What the world sees is a cryptographic proof; what the node keeps is only that signed proof.

---

## Why This Exists

EU law is switching on machine-readable Digital Product Passports sector by sector: battery passports become mandatory on **18 February 2027** (Reg. 2023/1542), the unsold-goods rules are in force **now** (ESPR Art. 24/25), detergents follow in 2029, and the ESPR working plan queues textiles, steel and more behind them. The eight European system standards (EN 18216–18246) published in 2026. No affordable, developer-friendly infrastructure exists for the millions of SMEs who need to comply.

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
      schemas/ .......... Versioned JSON Schemas for 11 sectors (battery, textile, electronics, …), embedded via include_str!
    dpp-crypto .......... Ed25519 keys, AES-256-GCM, JWS sign/verify, did:web DID builder, VCs, access policy engine
    dpp-digital-link .... GS1 Digital Link parser, link-type negotiation, AAS submodel mapping
    dpp-plugin-traits ... Wasm sector plugin ABI (no_std compatible, capability negotiation)
    dpp-plugin-sdk ...... Guest-side plugin SDK: export_plugin! macro + Validator
    dpp-rules ........... Pure no_std cross-field regulatory rules, shared by dpp-domain and plugins
    dpp-registry ........ EU Central Registry interface types (wasm32-safe)
    dpp-calc ............ EU-methodology calculators (CO2e, repairability), pure functions
    dpp-evidence ........ Evidence dossier wire format (DossierV1) + offline verification engine —
                          deliberately free of BSL-licensed and wasm-unsafe dependencies (spec/)
    dpp-tests ........... Cross-crate integration tests (domain + crypto + gs1)
  plugins/ .............. 10 Wasm sector plugins (wasm32-wasip1, excluded from workspace)
```

---

## Regulatory Coverage

| Regulation | Status | dpp-core Implementation |
|---|---|---|
| **ESPR** (EU 2024/1781) | In force; unsold-goods rules (Art. 24/25) apply since Jul 2026 | Core data model (Art. 9-13, Annex III), access rights per Art. 11(b), unsold-goods sector, transfer-of-responsibility design (not a distinct ESPR article — see below) |
| **Battery Regulation** (EU 2023/1542) | In force — passport mandatory **18 Feb 2027** | `BatteryData` struct, Annex XIII fields, sector schema |
| **Textile DPP Delegated Act** | Pending (ESPR working-plan priority) | `TextileData` with SVHC disclosure, per-fibre traceability, durability metrics — provisional until the act finalises |
| **CEN/CLC JTC 24 system standards** | **Published 2026** (EN 18216–18246; OJEU harmonisation citation pending) | Conformance tracked clause-by-clause; identifiers, carriers, API and authentication semantics aligned |
| **GS1 Digital Link v1.2** | Published | AI 01/21/10 parsing, link-type negotiation |
| **IDTA AAS Metamodel** | Published | DPP-to-AAS SubmodelElement mapping |
| **W3C VC Data Model v2.0** | Published | `DppAccessCredential` with role-based access tiers |

---

## Key Features

### Three-Tier Access Control (ESPR Art. 9(2)(f), Art. 11(b))

The three-tier split (Public/Professional/Confidential) is this project's own design — ESPR
requires per-actor access rights to be specified in each product group's delegated act (Art.
9(2), point (f)) and guarantees free, easy access based on those rights (Art. 11, point (b)); it
does not itself mandate exactly three tiers.

The access tier system gates DPP data based on W3C Verifiable Credentials:

- **Public** — Fibre composition, country of manufacturing, care instructions, environmental metrics. No credential required.
- **Professional** — SVHC substances, disassembly instructions, spare parts availability. Requires a VC proving role (repairer, recycler, remanufacturer).
- **Confidential** — Compliance reports, audit history, supply chain traceability. Requires an institutional DID (market surveillance authority, customs).

### Transfer of Responsibility

When a product undergoes remanufacturing, repurposing, or preparation for reuse, DPP responsibility transfers to the new economic operator. ESPR has no distinct "transfer of responsibility" article by that name — this design follows from the general data-accuracy duty (Art. 9(1): DPP data "shall be accurate, complete and up to date") and the registry-upload duty (Art. 13(4)). The `TransferChain` provides:

- Append-only provenance log with state machine validation
- DID-identified economic operators with typed roles
- Dual-signature transfer records (JWS from both parties)
- Rejection of invalid transfers (wrong operator, duplicate pending)

### Evidence Dossiers & Offline Verification

`dpp-evidence` defines a self-contained, signed **evidence dossier** (`DossierV1`) — passport, both JWS proofs, the issuer's DID document, the hash-chained audit trail, and the transfer chain in one canonical JSON file — plus the verification engine that checks all of it **fully offline**: no resolver, no network, no trust in Odal. An auditor, customs officer, or skeptical buyer runs the independent checks (manifest signature, content integrity, both JWS, audit-chain linkage, transfer signatures) and gets a named verdict per check. The crate is deliberately free of BSL-licensed and wasm-unsafe dependencies. Wire format specification: [`crates/dpp-evidence/spec/dossier-v1.md`](crates/dpp-evidence/spec/dossier-v1.md).

### Schema Validation

Versioned JSON schemas at `crates/dpp-domain/schemas/{sector}/v{version}.json` (embedded into the crate so they ship with it on publish):

| Sector | Versions | Key Fields |
|---|---|---|
| textile | v1.0.0, v1.1.0 | Fibre composition, SVHC, durability, microplastics |
| battery | v1.0.0, v2.0.0 | Chemistry, capacity, recycled content, state of health |
| electronics | v1.0.0 | Repairability, spare parts, substances of concern |
| steel | v1.0.0 | CO2 intensity, scrap content, production method |
| unsold-goods | v1.0.0 | Art. 25 destruction ban compliance |
| aluminium, construction, detergent, furniture, toy, tyre | v1.0.0 each | Sector-specific delegated-act fields |

The `VersionedSchemaRegistry` embeds schemas at compile time and supports runtime hot-reload for new versions.

### GS1 & Industry 4.0 Interoperability

- **Digital Link** — Full AI 01/21/10 parsing and building (GS1 URI Syntax v1.2)
- **Link-type Negotiation** — Content negotiation returning JSON, JSON-LD, HTML, or AAS representations
- **AAS Submodel Mapping** — Automatic conversion of DPP JSON to IDTA AAS SubmodelElement structures for Catena-X / Industry 4.0

### Wasm Sector Plugins

Compliance logic ships as sandboxed Wasm modules (`wasm32-wasip1`). Ten sector
plugins live under `plugins/` — battery (the reference implementation), textile,
electronics, steel, aluminium, construction, detergent, furniture, toy, and tyre.
Highlights:

| Plugin | Sectors | Key Rule |
|---|---|---|
| `sector-battery.wasm` | battery | Battery Regulation 2023/1542 (reference implementation) |
| `sector-textile.wasm` | textile, unsoldGoods | ESPR Art. 25 destruction ban (July 19, 2026) |
| `sector-steel.wasm` | steel | CBAM CO2e/tonne thresholds |

Plugin ABI supports capability negotiation and semantic versioning with compatibility checking.

---

## Port Traits

The 7 port traits define the core/platform boundary. Any downstream project implements these against its own infrastructure:

| Trait | Kind | Purpose |
|---|---|---|
| `PassportRepository` | async | CRUD for DPP records |
| `ComplianceRegistry` + `ComplianceStrategy` | sync | Sector-specific compliance dispatch |
| `IdentityPort` | async | Sign and verify passport JWS |
| `PluginHost` | sync | Wasm plugin dispatch |
| `ArchivePort` | async | Immutable DPP archival with retention guarantees |
| `RegistrySyncPort` | async | EU Central Registry registration and status sync |
| `SealPort` | async | eIDAS qualified electronic seal (ESPR Art. 13 / eIDAS 910/2014) |

---

## Quick Start

```bash
git clone https://github.com/odal-node/dpp-core.git
cd dpp-core

cargo build --workspace          # zero infrastructure needed
cargo nextest run --workspace    # full unit + integration suite
just check                       # fmt + clippy + test + audit
```

No Docker, no database, no env vars.

### Runnable Examples

```bash
cargo run -p dpp-domain --example create_passport                # Create & validate a textile DPP
cargo run -p dpp-crypto --example credential_and_transfer        # Issue a VC, transfer responsibility
cargo run -p dpp-digital-link --example gs1_and_aas              # Parse GS1 links, map to AAS submodel
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
| [dossier-v1.md](crates/dpp-evidence/spec/dossier-v1.md) | Evidence dossier wire-format specification (offline verification) |
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