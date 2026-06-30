# Changelog

All notable changes to dpp-core are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-07-01

A final-validation-review pass against authoritative regulatory and standards
sources (battery/ESPR citations, GS1, RFC 8037, IPCC AR6) found and fixed two
real defects, added one ESPR-driven capability, plus regression tests that
lock all three in. All crates bump together (workspace lockstep).

### Breaking

- **`PassportRepository::list` / `::count`** (`dpp-domain::ports::passport_repo`)
  gained a new `facility_id: Option<&str>` parameter, filtering to passports
  stamped with that exact ESPR Annex III facility identifier. Any external
  implementor of `PassportRepository` must add the parameter to its `list`/
  `count` impls. *Migration:* thread `facility_id` through (pass `None` to
  preserve prior unfiltered behaviour).
- **`unsold-goods` sector rename** — the sector previously keyed as
  `textile-unsold` (Rust type, schema directory, sector-catalog JSON, and the
  wire-format `sector` tag) is renamed to `unsold-goods`. *Migration:* update
  any code or stored data referencing the old `"textile-unsold"` key or the
  `crates/dpp-domain/schemas/textile-unsold/` path.
- **`electronics` schema `v1.0.0`** is **unchanged** (kept byte-identical to
  what `0.1.1` published, for historical immutability) — it still incorrectly
  declares `repairabilityScore` as a bare number, which the Rust
  `ElectronicsData` type never actually produced. A new **`electronics` schema
  `v1.1.0`** (now the catalog's `currentSchemaVersion`) corrects
  `repairabilityScore` to the structured `{overall, criteria}` shape that
  matches the Rust type. *Migration:* any caller validating electronics
  passports against the embedded schema should resolve the **current**
  version via `SectorCatalog::resolve_schema_version`/`VersionedSchemaRegistry`
  (as the write path already does) rather than hardcoding `v1.0.0`.

### Fixed

- Pinned the `Passport::retention_until` and `operator_identifier` ESPR
  citations against the verbatim OJ text of Regulation (EU) 2024/1781 — was
  previously `COMPLIANCE-PIN PENDING` (Art. 9(2)(i)/11(e) for the availability
  period, Art. 10(4) for the back-up-copy duty, Annex III point (k)/Art. 12
  for the responsible-operator identifier).
- Corrected `registry::TransferNotification`'s doc comment: ESPR has **no**
  distinct "transfer of responsibility" article; the prior "Article 9"
  citation was unconfirmed and is now attributed to its actual basis
  (Art. 9(1) general accuracy duty + Art. 13(4) registry-upload duty).
- README, `dpp-calc` README, `dpp-plugin-traits` doc comments, and the
  furniture/electronics schema descriptions no longer imply the repairability
  heuristic is a regulatory index (EN 45554 / EU 2023/1669) — it is an
  explicitly non-regulatory, transparent six-factor heuristic.

### Added

- `facility_id` filter on `PassportRepository::list`/`count` (see Breaking).
- `validate_strict` fail-closed unit test, pinned independently of any
  handler/service wiring (the contract the publish path relies on for Q-2).
- Per-sector Rust-type-to-JSON-schema conformance suite covering all 11
  sectors — round-trips a schema-valid instance constructed from each
  sector's Rust type through that sector's current embedded schema. This is
  the test class that would have caught the `electronics` schema bug above.
- Independent (non-`ed25519-dalek`) Ed25519 cross-verification of a produced
  JWS using `ring`, plus a full DID Core structural-validity test (referential
  integrity between `authentication`/`assertionMethod` and
  `verificationMethod`) on a realistic primary + archived + revoked keystore.
- Official GS1 Digital Link conformance vectors, sourced from §5 of the GS1
  Digital Link Standard — URI Syntax (Release 1.6.0, Ratified Mar 2025).

## [0.1.1] - 2026-06-27

- Added the `ComplianceRegistry` port (`dpp-domain::ports::compliance`) and
  `ComplianceResult`/`ComplianceStatus` types, wiring compliance computation
  into the passport pipeline (`Passport::compliance_result`).
- Added a `PassthroughComplianceRegistry` reference implementation
  (`dpp-domain::compliance::passthrough_registry`).
- Added the battery recycled-content cross-field rule
  (`dpp-rules::batteries::recycled_content`).
- Extended `dpp-plugin-traits` with the metric/compliance surface plugins use
  to report results back through the same pipeline.

## [0.1.0] - 2026-06-23

Initial public release of the dpp-core workspace: nine Rust crates
(`dpp-domain`, `dpp-crypto`, `dpp-digital-link`, `dpp-rules`, `dpp-calc`,
`dpp-plugin-traits`, `dpp-plugin-sdk`, `dpp-registry`, `dpp-tests`) plus ten
standalone Wasm sector plugins. All crates share version 0.1.0.

> **Downstream note.** The sector model (open/data-driven catalog), the Wasm
> plugin ABI, and the `Passport` struct reached their current shape during
> development. Platform consumers integrating against this first release should
> read **Downstream integration** below.

### Added

- **`SectorCatalog`** (`dpp-domain::catalog`) — open, data-driven catalog of EU
  ESPR sectors. One embedded JSON manifest per sector at
  `crates/dpp-domain/sectors/{key}.json`; runtime-extensible via `register()`.
  Each `SectorDescriptor` carries `status` (`RegulatoryStatus::InForce` /
  `Provisional`), legal basis, effective date, schema versions,
  `currentSchemaVersion`, product categories, `access_tiers`, and plugin binding.
- **Schema-version resolution** — `SectorCatalog::current_schema_version` /
  `resolve_schema_version(key, stored)` and `Sector::catalog_key()`. No call site
  hardcodes a version any more.
- **`dpp-plugin-sdk`** crate — `export_plugin!(P)` generates the full Wasm ABI
  (incl. a new `describe()` export returning `PluginCapabilities`) and a fluent
  `Validator`. Plugins implement `DppSectorPlugin` and stop hand-rolling the ABI.
- **`dpp-rules`** crate — pure, shared cross-field regulatory rules (fibre sum,
  SVHC, surfactant bands), consumed by both `dpp-domain` and the plugins.
- `AbiResult` envelope and `Clone` on `PluginError` (`dpp-plugin-traits`).
- `ComplianceStatus::NotAssessed` and `gate_determination(in_force, status)` —
  provisional sectors cannot surface a binding `Compliant`/`NonCompliant`.
- `SectorAccessPolicy::from_catalog(catalog, key)` — catalog-driven access policy
  for **every** sector (tiers declared in the manifests).
- New typed `ProductCategory` enum (product sub-types: `EvBattery`, `Apparel`,
  `Smartphone`, … `Other(String)`) and a `Passport.sector: Sector` field.
- All 10 sector plugins (battery, textile, steel, electronics, construction,
  tyre, toy, aluminium, furniture, detergent) on the SDK, each with real
  `validate_input` + per-`(sector,version)` compiled-schema cache in the registry.

### Design decisions (relevant to downstream integrators)

- **`Passport`** carries `sector: Sector` (required, the dispatch key) and
  `product_category: Option<ProductCategory>` (the typed sub-type). Wire format
  uses `"sector":"battery"`; `validate()` enforces
  `sector == sector_data.sector()` when sector data is present.
- **`ProductCategory`** is a product sub-type enum (`snake_case`, with
  `Other(String)`) — distinct from `Sector`. See `DATA-MODEL.md` §3.4.
- **Wasm plugin ABI** exports `alloc`/`dealloc`/`metadata`/`describe`/`validate`/
  `calculate_metrics`/`generate_passport`; the fallible calls return the
  `AbiResult` envelope (`{ok: …}` / `{error: PluginError}`). Hosts call
  `describe()` + `check_compatibility` before dispatch.
- `validate_sector_data` routes through `VersionedSchemaRegistry` at the
  catalog-resolved current version (battery validates against **v2.0.0**).
- `PassthroughRegistry` is sector-agnostic (handles all sectors uniformly). The
  `ComplianceRegistry` / `ComplianceStrategy` port remains the seam a
  plugin-backed or premium determination path wires into.
- `SectorAccessPolicy::from_catalog(catalog, key)` provides a catalog-driven
  access policy for every sector; `passport_default()` remains available.

### Fixed

- Battery **v2.0.0** schema existed on disk but was never embedded in the
  registry (so `latest("battery")` wrongly returned v1.0.0) — now embedded.
- Latent Wasm ABI bug: `write_output` allocates an exact-size buffer so the host's
  `dealloc(ptr, len)` is no longer a size-mismatched free.
- The textile fibre-sum rule was implemented twice (domain + plugin) — now one
  implementation in `dpp-rules`.
- `VersionedSchemaRegistry::register()` now rejects schemas that are valid JSON
  but do not compile as a JSON Schema (previously panicked at first `validate`).

### Downstream integration (platform)

Integration points for `dpp-engine` against this release — see the platform crates noted:

1. **`Passport` construction/read** (`dpp-vault`, `dpp-dal`, `dpp-integrator`): set
   `sector: Sector` (required) and `product_category: Option<ProductCategory>`;
   drop the old sector-as-category usage.
2. **`ProductCategory` matches** (`dpp-vault/handlers/create.rs`,
   `dpp-integrator/domain/validator.rs`): the old variants are gone; map request
   input to `Sector` for dispatch and (optionally) the new `ProductCategory`.
3. **Stored passports**: a data migration is needed — `"productCategory":"BATTERY"`
   → `"sector":"battery"` on the `passport` table.
4. **Plugin host** (`dpp-plugin-host`): parse the `AbiResult` envelope from
   `calculate_metrics`; optionally adopt `describe()` + `check_compatibility`; map
   the new `"NOT_ASSESSED"` plugin status to `ComplianceStatus::NotAssessed`; apply
   `gate_determination(catalog.is_in_force(key), status)`.
5. **Access policy** (`dpp-resolver/handlers/resolve_json.rs`): replace
   `battery_default()`/`textile_default()` with `from_catalog(&SectorCatalog::new(), key)`
   — this also covers all other sectors for free.
6. Optional: replace the platform's hand-rolled `sector_key()` with
   `Sector::catalog_key()`, and resolve schema versions via the catalog.

### Component summary

Per-crate inventory of what ships in this release.

#### dpp-domain

- `Passport`, `PassportId`, `Sector`, `SectorData` domain types.
- Typed sector data: `BatteryData`, `TextileData`, `SteelData`, `UnsoldGoodsReport`.
- `TransferChain` for modelling custody transfers along supply chains.
- `VersionedSchemaRegistry` with embedded JSON Schema validation (`schemas/` directory).
- Port traits defining the core/platform boundary: `PassportRepository`,
  `ComplianceRegistry`, `ComplianceStrategy`, `IdentityPort`, `PluginHost`,
  `ArchivePort`, `RegistrySyncPort`, `SealPort`.
- `SectorCatalog` (`dpp-domain::catalog`) — open, data-driven catalog of EU ESPR
  sectors with one embedded JSON manifest per sector; runtime-extensible via
  `register()`. Drives schema-version resolution and per-sector access policy.
- `DppError` unified error type.
- JSON schemas: battery v1.0.0, textile v1.0.0 + v1.1.0, steel v1.0.0, unsold-goods v1.0.0.
- `create_batch` and `update_batch` async methods on `PassportRepository` with
  default sequential fallback.
- `validate_sector_data_batch` and `batch_errors` helpers for bulk import pipelines.

#### dpp-crypto

- Ed25519 key generation, signing, and verification via `ed25519-dalek`.
- AES-256-GCM field-level encryption and decryption.
- JWS (JSON Web Signature) compact serialisation — sign and verify.
- `did:web` DID document builder.
- `LocalIdentityService` implementing `IdentityPort`.
- Verifiable Credential issuance and verification (W3C VC Data Model v2.0).
- Access-tier policy engine (Public / Professional / Confidential, ESPR Art. 10).

#### dpp-digital-link

- GS1 Digital Link URL parser (pure, no I/O).
- Link-type content negotiation.
- AAS (Asset Administration Shell) submodel mapping.

#### dpp-plugin-traits

- `DppSectorPlugin` trait for Wasm sector plugins (`no_std` compatible).
- Plugin versioning and capability negotiation types.
- `wasm32-wasip1` ABI boundary types.

#### dpp-registry

- EU Central Registry interface types (wasm32-safe): `RegistrationPayload`,
  `EuRegistryEnvelope`, `EuRegistryResponse`, `StatusResponse`,
  `TransferNotification`, the four Art. 13 identifier structs, and
  `RegistryEndpoint` with sandbox/production presets.

#### dpp-rules

- Pure `#![no_std]`, zero-dependency cross-field regulatory rules (fibre sum,
  SVHC disclosure, surfactant bands) shared by `dpp-domain` and the Wasm plugins.
- Active sector rule modules: batteries, electronics, textiles, chemicals.
  Construction, metals, and toys are placeholder modules pending delegated acts
  (see `docs/regulatory/REGULATORY.md`).

#### dpp-plugin-sdk

- Guest-side SDK with the `export_plugin!` macro, which generates the full Wasm
  ABI (incl. `describe()`) and wires each export to a `DppSectorPlugin` method.
- Fluent `Validator` helpers; re-exports `dpp-plugin-traits` as `traits` and
  `dpp-rules` as `rules`.

#### dpp-calc

- Pure, stateless EU-methodology calculators: cradle-to-gate CO₂e and a
  non-regulatory repairability heuristic, each emitting a `CalculationReceipt`.
- Date-based `ruleset_registry`; licensed LCI data injected at runtime via
  `FactorProvider` (never bundled). Battery CFB is a stub returning
  `NotImplemented` pending its delegated act.

#### dpp-tests

- Cross-crate integration suite (`publish = false`): textile end-to-end, transfer
  of responsibility, access-tier gatekeeping, schema conformity, all-sectors AAS,
  battery end-to-end, and adversarial security tests.

#### Sector Plugins (standalone, not in workspace)

Ten Wasm sector plugins on the SDK (`dpp-plugin-sdk` + `export_plugin!`);
`sector-battery` is the reference implementation:

- `sector-battery` — Battery Regulation 2023/1542.
- `sector-textile` — Textile / ESPR (incl. Art. 25 / Annex VII unsold-goods).
- `sector-steel` — Steel / CBAM carbon-intensity.
- `sector-electronics`, `sector-aluminium`, `sector-construction`,
  `sector-detergent`, `sector-furniture`, `sector-toy`, `sector-tyre`.

#### Examples

- Three runnable usage examples (`create_passport`, `credential_and_transfer`,
  `gs1_and_aas`) as a workspace member crate. Demonstrates passport creation, schema
  validation, VC issuance, transfer chains, GS1 parsing, and AAS mapping.

### Cross-crate consolidation

- `AccessTier` has a single canonical definition in `dpp-domain`, used by both
  `dpp-crypto` and `dpp-digital-link` (rather than duplicated in each).
- `dpp-domain` re-exports `SteelData`, `UnsoldGoodsReport`, `UnsoldGoodsDestination`,
  `UnsoldGoodsReason` from crate root.
- `dpp-digital-link` now depends on `dpp-domain` for the canonical `AccessTier` type.
- `Passport` gains `transition_to()` method that enforces the status state machine
  and returns `DppError::InvalidTransition` on illegal transitions. Sets
  `retention_locked = true` and `published_at` on first publish.
- Added `#[must_use]` annotations to key constructors and builders.

[0.1.0]: https://github.com/odal-node/dpp-core/releases/tag/v0.1.0
