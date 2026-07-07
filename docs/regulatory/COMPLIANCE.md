# Regulatory Compliance Transparency

This document maps dpp-core's functionality to the EU regulations it
implements, tracks the standards it aligns with, and describes how regulatory
changes are incorporated.

## Regulatory Scope

dpp-core provides the domain types and validation logic for Digital Product
Passports mandated by:

| Regulation | Reference | Status | dpp-core Coverage |
|---|---|---|---|
| ESPR | Regulation (EU) 2024/1781 | In force | Core passport model, access tiers (Art. 10), schema validation |
| Battery Regulation | Regulation (EU) 2023/1542 | In force, DPP deadline Feb 2027 | Battery schemas (v1.0.0, v2.0.0), sector plugin |
| Textile (anticipated) | ESPR delegated act (draft) | See timeline note below | Textile schemas (v1.0.0, v1.1.0), sector plugin |
| CBAM | Regulation (EU) 2023/956 | In force | Steel schema (v1.0.0), embedded emissions fields |

> **Timeline note:** the textile DPP delegated act has no adopted date yet; published estimates range from ~2026 to 2028. Treat any specific year here as provisional and verify against EUR-Lex before relying on it.

dpp-core does **not** implement infrastructure concerns (HTTP APIs, databases,
registry connectivity). Those belong in the platform layer.

## Standards Alignment

| Standard | Body | Version Tracked | Where Used |
|---|---|---|---|
| GS1 Digital Link | GS1 | v1.2 | `dpp-digital-link` — URL parsing and link-type negotiation |
| AAS (Asset Administration Shell) | IDTA | v3.0 | `dpp-digital-link` — submodel mapping |
| W3C Verifiable Credentials | W3C | Data Model v2.0 | `dpp-crypto` — VC issuance and verification |
| DID:web | W3C | did:web Method Spec | `dpp-crypto` — DID document builder |
| JSON Schema | IETF | Draft 2020-12 | `dpp-domain` — passport data validation |
| CEN/CENELEC JTC 24 | CEN | Draft (monitoring) | Schema structure anticipated to align |

## Schema-to-Regulation Mapping

Each JSON schema under `schemas/` is traceable to its regulatory basis:

| Schema File | Regulation | Key Articles/Annexes | Notes |
|---|---|---|---|
| `battery/v1.0.0.json` | 2023/1542 | Art. 77, Annex XIII | Minimum viable battery passport |
| `battery/v2.0.0.json` | 2023/1542 | Art. 77, Annex XIII | Extended fields for carbon footprint |
| `textile/v1.0.0.json` | ESPR 2024/1781 | Art. 9-10 (framework) | Baseline textile passport |
| `textile/v1.1.0.json` | ESPR 2024/1781 | Art. 9-10 + anticipated delegated act | Adds fibre composition, durability, microplastics |
| `unsold-goods/v1.0.0.json` | ESPR 2024/1781 | Art. 25 / Annex VII | Unsold-goods destruction-ban compliance |
| `steel/v1.0.0.json` | CBAM 2023/956 | Art. 7 | Embedded emissions, production origin |
| `electronics/v1.0.0.json` | ESPR 2024/1781 | (delegated act anticipated) | Repairability, spare parts, substances |
| `aluminium/v1.0.0.json` | ESPR 2024/1781 / CBAM | (delegated act anticipated) | Production route, CO₂e/tonne, recycled content |
| `construction/v1.0.0.json` | CPR 2024/3110 | (delegated acts anticipated) | Construction product fields |
| `detergent/v1.0.0.json` | ESPR 2024/1781 | (delegated act anticipated) | Surfactant / ingredient fields |
| `furniture/v1.0.0.json` | ESPR 2024/1781 | (delegated act anticipated) | Furniture sector fields |
| `toy/v1.0.0.json` | EU 2025/2509 (Toy Safety) | (delegated act anticipated) | Toy sector fields |
| `tyre/v1.0.0.json` | ESPR 2024/1781 | (delegated act anticipated) | Tyre sector fields |

The current schema version per sector is resolved by `SectorCatalog`, not hardcoded at call sites. See `docs/regulatory/REGULATORY.md` for which sectors carry implemented compliance rules vs. placeholders.

## Regulatory Change Process

When an EU regulation or delegated act is published or amended:

1. **Monitor** — Track publications via EUR-Lex and JTC 24 mailing lists.
   This is recorded in the project roadmap.
2. **Assess** — Open a GitHub issue tagged `regulatory` describing the change
   and its impact on dpp-core's schemas, domain types, or compliance rules.
3. **Implement** — Create a new schema version (never modify existing ones).
   Update domain types if required. All changes go through the standard PR
   process.
4. **Document** — Update this file's mapping tables. Add a CHANGELOG entry
   referencing the regulation.
5. **Release** — Cut a new version per the [release process](../governance/RELEASE.md).

### Schema Immutability Rule

Published schema versions are **never modified**. If a regulation changes
requirements, a new version is created. This ensures that passports validated
against v1 remain valid against v1 indefinitely, even after v2 is published.

## Compliance Architecture

dpp-core exposes a pluggable determination seam for sector-specific compliance:

```
ComplianceRegistry (port trait)
  ├── PassthroughRegistry (dpp-core, Apache) → PassthroughNoValidation, computes nothing
  └── plugin-backed registry (platform)      → Wasm sector plugins (sector-battery, etc.)
```

The `ComplianceRegistry` trait defines `compute(&self, sector: Sector, data:
&SectorData) -> Result<ComplianceResult, ComplianceError>`; the per-sector
`ComplianceStrategy` trait defines `compute(&self, data: &SectorData) ->
Result<ComplianceResult, ComplianceError>`. The Apache default
(`PassthroughRegistry`) computes nothing and returns
`PassthroughNoValidation` for every sector; real determinations come from the
Wasm sector plugins (or a proprietary `PremiumComplianceRegistry`). A computed
determination is passed through `gate_determination(catalog.is_in_force(sector),
…)` so a provisional sector can never surface a binding result. This separation
means:

- **Core stays generic** — no sector-specific determination logic in the workspace crates.
- **Regulations are isolated** — a Battery Regulation change only touches
  `sector-battery` (and the shared rules in `dpp-rules`).
- **New sectors are additive** — adding a new delegated act means adding a
  schema file, a catalog manifest, and a plugin.

## Cryptographic Compliance

| Primitive | Algorithm | Library | Purpose | Regulatory Basis |
|---|---|---|---|---|
| Signing | Ed25519 | `ed25519-dalek` | Passport authenticity, JWS | ESPR Art. 10 (data integrity) |
| Encryption | AES-256-GCM | `aes-gcm` | Confidential field protection | ESPR Art. 10(3) (access tiers) |
| Hashing | SHA-256 | `sha2` | Data fingerprinting | General integrity |
| Entropy | OS CSPRNG | `rand` | Key generation | Cryptographic best practice |

These choices are documented in [SECURITY.md](../../SECURITY.md) and
the architecture docs under `docs/architecture/IDENTITY.md`.

## EU Registry Readiness

The EU DPP Registry (ESPR Article 13) will define an API for passport
registration and lookup; the official specification was not yet published as of
this release. dpp-core's `dpp-registry` crate is a **ghost connector** carrying
preparatory interface types:

- `RegistrationPayload`, `EuRegistryEnvelope`, `EuRegistryResponse`,
  `StatusResponse`, `TransferNotification`, the four Art. 13 identifier structs
  (`ProductIdentifier`, `ProductItemIdentifier`, `FacilityIdentifier`,
  `OperatorIdentifier`), error types, and `RegistryEndpoint` — anticipated data
  shapes based on published ESPR articles and JTC 24 draft discussions.
- `RegistrySyncPort` — the port trait (defined in `dpp-domain::ports`, with a
  `GhostRegistrySync` placeholder) that the platform implements once the
  official API specification is released.

These types are explicitly unstable and will be updated when the official
specification is published. (The expected go-live date has shifted; verify
against EUR-Lex / the Commission before relying on any specific date.)

## Transparency Commitments

1. **All compliance-relevant code is open-source** under Apache-2.0.
2. **Schema validation is deterministic** — the same input always produces the
   same validation result, regardless of platform or runtime.
3. **No vendor lock-in** — dpp-core has zero infrastructure dependencies. Any
   platform that implements the port traits can use it.
4. **Audit trail** — every compliance rule change is tracked in version
   control with a link to the originating regulation.

## References

- [ESPR Regulation (EU) 2024/1781](https://eur-lex.europa.eu/eli/reg/2024/1781)
- [Battery Regulation (EU) 2023/1542](https://eur-lex.europa.eu/eli/reg/2023/1542)
- [CBAM Regulation (EU) 2023/956](https://eur-lex.europa.eu/eli/reg/2023/956)
- [GS1 Digital Link Standard](https://www.gs1.org/standards/gs1-digital-link)
- [W3C Verifiable Credentials Data Model v2.0](https://www.w3.org/TR/vc-data-model-2.0/)
- [SECURITY.md](../../SECURITY.md)
- [VERSIONING.md](../governance/VERSIONING.md)
