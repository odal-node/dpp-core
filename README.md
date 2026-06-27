# Odal Node Core

**EU Digital Product Passport Standard Library**

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/odal-node/dpp-core/actions/workflows/ci.yml/badge.svg)](https://github.com/odal-node/dpp-core/actions/workflows/ci.yml)
[![Rust 1.96+](https://img.shields.io/badge/Rust-1.96%2B-orange.svg)](https://www.rust-lang.org/)

Pure, stateless Rust library for EU ESPR Digital Product Passport compliance. Domain types, cryptographic signing (Ed25519 + JWS), W3C Verifiable Credentials, GS1 Digital Link resolution, schema validation, and AAS submodel mapping. No database, no HTTP framework, no infrastructure dependencies.

Anyone building DPP tooling can use this library as the foundation. It is the standard, not the product.

> Note: This project is in active development.

> *"We provide the pipe, not the truth."* Odal Node uses a **proof-bound architecture**: product data is validated locally, signed with your key, and published as a verifiable passport — the raw inputs are discarded after signing, never retained by the node. What the world sees is a cryptographic proof; what the node keeps is only that signed proof.

---

## Why This Exists

By 2027, every product sold in the EU requires a machine-readable Digital Product Passport. The first delegated acts for batteries and textiles are already in force. No affordable, developer-friendly infrastructure exists for the millions of SMEs who need to comply.

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
    dpp-tests ........... Cross-crate integration tests (domain + crypto + gs1)
  plugins/ .............. 10 Wasm sector plugins (wasm32-wasip1, excluded from workspace)
```

---

## Regulatory Coverage

| Regulation | Status | dpp-core Implementation |
|---|---|---|
| **ESPR** (EU 2024/1781) | In force | Core data model (Art. 8-13), three-tier access (Art. 10), transfer of responsibility (Art. 12) |
| **Battery Regulation** (EU 2023/1542) | In force | `BatteryData` struct, Annex VII fields, sector schema |
| **Textile DPP Delegated Act** | Anticipated 2025-2026 | `TextileData` with SVHC disclosure, per-fibre traceability, durability metrics |
| **JTC 24 Data Standard** | Draft (CEN/CENELEC) | Schema fields track latest published draft |
| **GS1 Digital Link v1.2** | Published | AI 01/21/10 parsing, link-type negotiation |
| **IDTA AAS Metamodel** | Published | DPP-to-AAS SubmodelElement mapping |
| **W3C VC Data Model v2.0** | CR | `DppAccessCredential` with role-based access tiers |

---

## Key Features

### Three-Tier Access Control (ESPR Art. 10)

The access tier system gates DPP data based on W3C Verifiable Credentials:

- **Public** — Fibre composition, country of manufacturing, care instructions, environmental metrics. No credential required.
- **Professional** — SVHC substances, disassembly instructions, spare parts availability. Requires a VC proving role (repairer, recycler, remanufacturer).
- **Confidential** — Compliance reports, audit history, supply chain traceability. Requires an institutional DID (market surveillance authority, customs).

### Transfer of Responsibility (ESPR Art. 12)

When a product undergoes remanufacturing, repurposing, or preparation for reuse, DPP responsibility transfers to the new economic operator. The `TransferChain` provides:

- Append-only provenance log with state machine validation
- DID-identified economic operators with typed roles
- Dual-signature transfer records (JWS from both parties)
- Rejection of invalid transfers (wrong operator, duplicate pending)

### Schema Validation

Versioned JSON schemas at `crates/dpp-domain/schemas/{sector}/v{version}.json` (embedded into the crate so they ship with it on publish):

| Sector | Versions | Key Fields |
|---|---|---|
| textile | v1.0.0, v1.1.0 | Fibre composition, SVHC, durability, microplastics |
| battery | v1.0.0, v2.0.0 | Chemistry, capacity, recycled content, state of health |
| electronics | v1.0.0 | Repairability, spare parts, substances of concern |
| steel | v1.0.0 | CO2 intensity, scrap content, production method |
| textile-unsold | v1.0.0 | Art. 22 destruction ban compliance |
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
| `sector-textile.wasm` | textile, textileUnsoldGoods | ESPR Art. 22 destruction ban (July 19, 2026) |
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
cargo run --example create_passport           # Create & validate a textile DPP
cargo run --example credential_and_transfer   # Issue a VC, transfer responsibility
cargo run --example gs1_and_aas              # Parse GS1 links, map to AAS submodel
```

---

## Integration Tests

| Test Suite | What It Validates |
|---|---|
| `textile_end_to_end` | Full passport lifecycle: creation, serialisation, AAS mapping, GS1 parsing, credential issuance, access filtering |
| `transfer_of_responsibility` | Transfer chain lifecycle, provenance audit trail, error cases, serialisation |
| `access_tier_gatekeeping` | All three ESPR tiers with realistic credentials, edge cases, custom policies |
| `schema_conformity` | JTC 24 field coverage, SVHC structure, ISO 3166 enforcement, schema strictness |

---

## Documentation

| Document | Description |
|---|---|
| [BLUEPRINT.md](docs/project/BLUEPRINT.md) | Project vision, guiding principles, non-goals |
| [ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md) | Core library architecture and module design |
| [DATA-MODEL.md](docs/architecture/DATA-MODEL.md) | DPP canonical schema (ESPR / Battery Regulation aligned) |
| [IDENTITY.md](docs/architecture/IDENTITY.md) | `did:web` and Verifiable Credential deep dive |
| [PLUGIN-HOST.md](docs/architecture/PLUGIN-HOST.md) | Wasm plugin sandbox design and ABI contract |
| [PROOF-BOUND-ARCHITECTURE.md](docs/architecture/PROOF-BOUND-ARCHITECTURE.md) | Data sovereignty, signed proof relay model, and passport retention guarantees |
| [DESIGN-PATTERNS.md](docs/architecture/DESIGN-PATTERNS.md) | Hexagonal architecture, open-core boundary patterns |
| [CONFORMITY.md](docs/regulatory/CONFORMITY.md) | Regulatory alignment statement for assessment bodies |
| [CONTRIBUTING.md](docs/governance/CONTRIBUTING.md) | Contributor guide: setup, conventions, PR workflow |
| [SECURITY.md](docs/project/SECURITY.md) | Vulnerability disclosure policy |


## License

[Apache License 2.0](LICENSE)

## Security

Do **not** open public issues for security vulnerabilities. Report privately to **security@odal-node.io** — see [SECURITY.md](docs/project/SECURITY.md) for full disclosure policy.

---

*Built by [Odal Node](https://odal-node.io)