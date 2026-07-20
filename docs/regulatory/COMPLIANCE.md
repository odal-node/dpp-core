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

The EU DPP Registry (ESPR Article 13) **became operational on 20 July 2026**,
together with a testing environment and User Guidelines. Its operating rules are
Commission Implementing Regulation (EU) 2026/1778.

Verified against the OJ text of IR 2026/1778 (adopted 16 July 2026, published
17 July 2026, in force on the twentieth day thereafter — **Art. 24**):

- **Who may register.** A digital product passport is registered by a *verified
  economic operator* placing the product on the market (**Art. 8(1)**).
  Verification is by eIDAS credential: for a legal person, a **qualified
  electronic seal** supported by a qualified certificate issued by a QTSP, or a
  qualified electronic attestation of attributes (**Art. 4(2)**). Establishment
  in the Union is **not** a precondition — Art. 4(2)(b) provides expressly for
  operators not required to be so established.
- **Registration by a third party is permitted.** Where a verified economic
  operator authorises a third party to perform registration actions on its
  behalf, that third party must itself complete the verification process of
  **Art. 5** (value chain actors — which the recitals enumerate as including a
  digital product passport service provider). The economic operator "shall
  remain fully responsible for compliance with the obligations set out in this
  Regulation" (**Art. 19(4)**) and is the controller of the data it submits
  (**Art. 19(5)**). Delegating the mechanics does not move the liability.
- **Registry structure** (**Art. 3**) includes an API for registering passports
  and retrieving information (Art. 3(b)), a list of verified digital product
  passport service providers (Art. 3(f)), and a storage component for unique
  identifiers *and commodity codes for products intended to be placed under the
  customs procedure "release for free circulation"* (Art. 3(e)).
- **Granularity.** Registration occurs at model, batch or item level as the
  applicable delegated act requires (**Art. 8(1)**); where rules conflict, at
  the most granular level required (**Art. 8(3)**). An item-level passport must
  link **both** batch and model identifiers where those exist (**Art. 8(4)**);
  a batch-level passport must link the model identifier (**Art. 8(5)**).
- **Automated checks on submission** (**Art. 8(7)**): semantic conformity,
  coherence of mandatory data, conformity with the required granularity level,
  and — *where relevant* — validity of the commodity code.

dpp-core's `dpp-registry` crate is a **ghost connector** carrying preparatory
interface types that **predate the published specification**:

- `RegistrationPayload`, `EuRegistryEnvelope`, `EuRegistryResponse`,
  `StatusResponse`, `TransferNotification`, the four Art. 13 identifier structs
  (`ProductIdentifier`, `ProductItemIdentifier`, `FacilityIdentifier`,
  `OperatorIdentifier`), error types, and `RegistryEndpoint` — anticipated data
  shapes based on published ESPR articles and JTC 24 draft discussions.
- `RegistrySyncPort` — the port trait (defined in `dpp-domain::ports`, with a
  `GhostRegistrySync` placeholder) that the platform implements once the
  official API specification is released.

These types remain explicitly unstable, and are **known to diverge** from the
published specification rather than merely being provisional. Divergences
confirmed against the OJ text:

- **Commodity code** is absent from these types and from the passport model
  entirely. It is stored by the registry for products intended for the customs
  procedure "release for free circulation" (Art. 3(e)) and validated *where
  relevant* on submission (Art. 8(7)(d)) — so it is conditional on the customs
  path, not universal, but unrepresentable for us today either way.
- **Registration granularity** — the specification requires model, batch or item
  level with the corresponding identifiers linked (Art. 8(1), (4), (5));
  `RegistrationPayload` carries an unconditional item identifier and models no
  granularity or identifier-linking concept at all.
- **Authentication** — `EuRegistryEnvelope` anticipates a bearer-token
  mechanism; registration rests on eIDAS verified-operator identity instead
  (Arts. 4–5). This is a structural mismatch, not a wrong endpoint.

Reconciling these is a breaking change to a core crate and is scheduled for the
next minor. Do not treat the current shapes as an implementation target.

## Transfer-of-Responsibility Article Pin

Verified against the OJ text of Regulation (EU) 2024/1781, 2026-07-04, to
resolve an internal citation ambiguity (the transfer-of-responsibility
obligation had been cited inconsistently as either Art. 9 or Art. 12):

- **No single article establishes a transfer-of-responsibility mechanism**
  for a DPP moving between economic operators (resale, recycler take-over,
  insolvency succession, etc.).
- **Art. 11(e)** is the closest fit: it requires the passport to "remain
  available ... including after an insolvency, a liquidation or a cessation
  of activity ... of the economic operator responsible for the creation of
  the digital product passport" — a continuity/availability obligation, not
  a transfer-mechanics one.
- **Art. 10(4)** is the adjacent back-up-copy obligation (via a DPP service
  provider), already cited above.
- **Art. 9** establishes no transfer *mechanism*, but it is not silent
  either: alongside the placing-on-market gate, **Art. 9(1)** requires that
  passport data "shall be accurate, complete and up to date" — the standing
  duty that makes a stale post-transfer passport non-compliant. That duty,
  together with the registry-upload duty (**Art. 13(4)**), is the narrow
  basis cited in `docs/regulatory/CONFORMITY.md` and
  `dpp-registry::registry::transfer`; it stands and is not superseded here.
- **Art. 12** (unique-*identifier* issuance mechanics, not registry upload)
  does not address transfer at all — the "Art. 12" leg of the earlier
  "Art. 9/12" citation was not traceable to operative text and is superseded
  by this entry.

Given no article mandates transfer mechanics, `domain::transfer`'s
dual-signed transfer handshake is a design choice that satisfies — and
exceeds — Art. 11(e)'s continuity requirement; it is not a literal
implementation of a numbered transfer obligation, because none exists.
Treat this as engineering due diligence, not legal advice — verify
independently before relying on it in a filing or contract.

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
