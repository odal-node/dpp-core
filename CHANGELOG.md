# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/) under
the pre-1.0 conventions in [VERSIONING.md](docs/governance/VERSIONING.md): a
**minor** bump may contain breaking changes, each listed below under a
**Breaking** heading with a migration note.

This file was started retroactively on 2026-07-03 at v0.4.0; entries for
0.1.0–0.3.0 are reconstructed from git history.

## [Unreleased]

## [0.8.0] - 2026-07-13

### Added

- **Cross-operator passport references (`PassportRef`).** A resolvable,
  hash-pinned reference to another operator's passport
  (`dpp-domain::domain::passport::PassportRef`: `{ uri, publicJwsHash }`).
  `dpp-domain::Passport` builds two edges on it — `parent_passport_ref`
  (second-life successor lineage) and `component_refs` (bill of materials).
- **BOM cycle-detection graph (`dpp-domain::domain::graph`).** A pure, bounded
  reachability check (`check_edge`) that refuses a component edge which would
  close a cycle or exceed the depth cap (`DEFAULT_DEPTH_CAP = 6`) over the local
  graph; cross-operator cycle safety stays a verify-time concern.
- **Read-time schema upcast lenses (`dpp-domain::schemas::lens`).** Pure,
  versioned `v_n → v_m` transforms so an old signed record stays byte-identical
  yet remains consumable by new-schema readers. `LensRegistry` composes
  multi-hop chains and refuses downcasts / missing hops with a typed error;
  ships the battery `1.0.0 → 2.0.0` lens (derives `ratedEnergyWh` from
  `ratedCapacityKwh`). Upcast only — the past can read the future never.
- **Lineage / BOM linkset relations.** `dpp-digital-link::Gs1LinkType` gains
  first-class `Predecessor` / `Successor` and `HasComponent` / `IsComponentOf`
  variants under Odal's own vocabulary namespace (GS1 defines no lineage or BOM
  relation).
- Property-based test harness (`proptest`) plus a `cargo-fuzz` scaffold for the
  GS1 Digital Link parser.

New **`dpp-rules::lint`** module: a non-binding plausibility lint pack —
arithmetic and physical-plausibility checks distinct from the crate's binding
regulatory rules (e.g. `ratedEnergyWh` vs. `nominalVoltageV × nominalCapacityAh`,
material-composition sums, implausible date/range values). First pack covers
battery, textile, and unsold-goods (15 lints total); each finding carries a
`LintSeverity::{Warning,Notice}` and is phrased as a question, never a
verdict. `no_std`, wasm32-safe. `dpp-domain::Passport` gains
`lint_result: Option<LintResult>` plus a `lint_sector_data()` dispatcher and
`LintResult::compute()` adapter mapping `SectorData` onto the pack.

### Breaking

- **`dpp-domain::Passport` gains `parent_passport_ref: Option<PassportRef>` and
  `component_refs: Vec<PassportRef>`.** Both are `#[serde(default)]`, so existing
  serialized passports deserialize unchanged (absent → `None` / empty) — no data
  migration. *Migration:* Rust code constructing a `Passport` via a struct
  literal must add the two fields (`parent_passport_ref: None,
  component_refs: Vec::new()`).
- **`dpp-digital-link::Gs1LinkType` gains new variants** (`Predecessor`,
  `Successor`, `HasComponent`, `IsComponentOf`). The enum is not
  `#[non_exhaustive]`, so a downstream exhaustive `match` on it must add arms (or
  a `_`).

### Removed

- **`dpp-evidence`** removed from the workspace and dissolved into
  `dpp-engine` — the evidence dossier format and its verification engine are
  a DB-backed engine feature, not a core standard. `AuditEntry` and its hash-chain algorithm
  (`chain_hash`/`verify_audit_chain`) return to `dpp-engine`'s `dpp-types`
  crate. No other published core crate depended on `dpp-evidence`.

## [0.7.0] - 2026-07-07

New crate **`dpp-evidence`** — the evidence dossier wire format and offline
verification engine (`DossierV1`, `verify_dossier_json`). Deliberately free
of BSL-licensed and wasm-unsafe dependencies.

Audit trail wire type (`AuditEntry`) and its hash-chain algorithm
(`chain_hash`/`verify_audit_chain`) promoted from `dpp-engine`'s
`dpp-types` crate into `dpp-evidence::audit` — the shape is
third-party-verifiable, making it part of the standard rather than engine
plumbing (the `dpp-types` module doc had flagged this as a "core-candidate"
since the hash-chain format was introduced). `dpp-types::audit` now
re-exports the type and keeps only the engine-side `AuditRepository`
persistence port.

Every dossier-owned type now rejects unknown fields at deserialization
(`deny_unknown_fields`), and `verify_dossier_json` adds a 9th check,
`input_fidelity`, comparing canonical input bytes against the canonical
bytes of what was actually parsed — this catches content silently dropped
inside tolerant nested types (e.g. an unknown field inside a `dpp-domain`
`TransferRecord`) that field-level strictness alone can't reach. See
`dpp-evidence/spec/dossier-v1.md` for the full format specification.

Golden cross-tests (`dpp-tests/tests/jws_cross_verification.rs`) now pin
agreement between `dpp-crypto`'s JWS signer/verifier and `dpp-evidence`'s
vendored copy across a sign/verify round trip, a full tamper matrix, and
key-rotation resolution — the drift guard for the one deliberately
duplicated piece of logic in the two crates.

### Breaking
- `IdentityPort` gains a new required method, `own_did_document(&self) ->
  Result<serde_json::Value, DppError>` — fetches the implementor's own
  current `did:web` document. *Migration:* every `IdentityPort`
  implementation (production adapters and test doubles alike) must add this
  method; `dpp-crypto::LocalIdentityService` and the HTTP client adapter in
  `dpp-vault` both already do.
- `AuditEntry::new`'s signature changes from taking `&AuthContext` to taking
  a plain `actor: impl Into<String>` — the type moved to `dpp-evidence`,
  which cannot depend on the engine-only `AuthContext`. *Migration:*
  replace `AuditEntry::new(id, action, auth, prev, new)` with
  `AuditEntry::new(id, action, &auth.user_id, prev, new)` at every call site.

## [0.6.0] - 2026-07-06

Internal re-layout of all 8 published crates: every `mod.rs` becomes a pure
index (module docs + `pub use` + `mod` decls only), each public type/trait
moves to its own file. Mechanically enforced going forward by a new CI
tripwire (`dpp-tests::mod_rs_is_pure_index`). The split is path-transparent
everywhere except the one entry below — `cargo public-api` confirms
`dpp-engine` builds and passes its full test suite against this tree with
**zero source changes** on the engine side.

### Breaking
- `dpp-domain::schemas::registry` is removed. The type it held,
  `VersionedSchemaRegistry`, is renamed to live in `schemas::versioned`
  (private module) and re-exported directly at `schemas::VersionedSchemaRegistry`
  — one level shallower than before. *Migration:* replace
  `dpp_domain::schemas::registry::VersionedSchemaRegistry` with
  `dpp_domain::schemas::VersionedSchemaRegistry` (already the path used by
  every internal and engine-side caller, so most consumers need no change).

### Changed
- `dpp-domain`: `passport`, `transfer`, `sector`, `validation`, and `catalog`
  each split into one file per type. The three no-op ("ghost") port
  implementations moved out of the port trait files into a private
  `ports::ghosts` module, re-exported at each port's original path
  (`ports::archive::GhostArchive`, `ports::registry_sync::GhostRegistrySync`,
  `ports::seal::GhostSeal`) — no import changes for existing callers.
- `dpp-crypto`: `keystore` split into `entry`/`store`; credential construction
  moved to `identity::build_passport_credential`.
- `dpp-digital-link`: `digital_link` split into `link`/`gtin`/`qr`; the AAS
  sector dispatch extracted to `aas::sectors::dispatch`.
- `dpp-rules`: the ruleset-bundle format and verification promoted to a
  `bundle` module, gated behind a new `bundle` feature so the crate's
  default (no-feature) build stays `no_std`.
- `dpp-calc`: CO2e and repairability calculators extracted out of their
  `mod.rs` files; repairability thresholds split into one file per concrete
  ruleset (`smartphone`, `laptop`, `displays`, `washing_machine`).
- `dpp-registry`: the single 915-line `registry.rs` split into a `registry/`
  module, one file per concern.
- `dpp-plugin-traits`: `lib.rs` split into `version`/`meta`/`result`/
  `plugin`/`error`.
- `dpp-plugin-sdk`: `lib.rs` split into `abi`/`codec`/`entry`.
- Governance docs (`CLAUDE.md`, `CONTRIBUTING.md`, `GOVERNANCE.md`,
  `SECURITY.md`) consolidated from `docs/agent/`, `docs/governance/`, and
  `docs/project/` to the repo root; the duplicate
  `docs/governance/CHANGELOG.md` removed in favor of this file.

### Added
- `bundle_version: Option<String>` on `dpp-calc::CalculationReceipt`
  (`with_bundle_version` builder) — the signed Compliance-Current bundle
  version that delivered the ruleset, `None` for the built-in baseline
  rulesets. Additive, non-breaking.

## [0.5.0] - 2026-07-04

### Breaking
- `Passport` gains a `seal: Option<SealedEnvelope>` field (`domain::passport`),
  carrying the eIDAS qualified electronic seal applied to the passport, if
  any (`placeholder: true` on the envelope means no legally valid seal
  exists yet). `Passport` is not `#[non_exhaustive]`, so any code
  constructing a `Passport` struct literal directly must add a `seal` field
  (typically `seal: None`); code that only reads existing fields is
  unaffected.

## [0.4.1] - 2026-07-03

### Added
- `homepage = "https://odal-node.io"` on the workspace and all 8 published
  crates, so crates.io shows a Homepage link.
- crates.io/docs.rs/license badges on each published crate's own README (the
  one crates.io renders on the crate page), matching the badge style already
  used in the root README.

## [0.4.0] - 2026-07-03

### Added
- `PassportStatus::Deactivated`: a terminal end-of-life status, reachable from
  `Published` or `Suspended` (`domain::status`). The enum is `#[non_exhaustive]`,
  so this is additive, not breaking, for downstream matches.
- `domain::eol` module: `EolEvent`, `DeactivationReason`
  (`Recycled` / `Destroyed { derogation }` / `Exported` / `Lost`), and
  `DerogationRef` — destroying a product must cite a recognised derogation
  from the ESPR Art. 25 unsold-goods destruction ban.
- `TransferRecord::signing_payload()` (`domain::transfer`): the canonical
  content both operators sign over in a transfer-of-responsibility handshake.
- A port-inventory drift tripwire test (`dpp-tests`) asserting
  `dpp-domain::ports` modules match `docs/architecture/PORTS.md`.

### Changed
- `ports::seal` rustdoc no longer states qualified-seal registration as
  enacted law; hedged as COMPLIANCE-PIN-PENDING until the EU registry API
  spec is published.
- Corrected stale ESPR article citations in `docs/architecture/OVERVIEW.md`
  and the `sector-textile` plugin metadata.
- Removed a stale integration-test table and a dead architecture-doc link
  from the root README.

## [0.3.0] - 2026-07-01

### Breaking
- `Passport.facility_id` replaced by a self-contained `FacilitySnapshot` so a
  published passport survives facility retirement. Consumers reading
  `facility_id` directly must migrate to the snapshot fields.

## [0.2.0] - 2026-07-01

### Breaking
- Sector key `textile-unsold` renamed to `unsold-goods`. Any code matching or
  serialising the old key must update to the new one.
- Electronics `repairabilityScore` schema corrected (wrong type) and versioned
  as new schema `v1.1.0`; consumers pinned to `v1.0.0` are unaffected, new
  data should target `v1.1.0`.

### Added
- `facility_id` filter on `PassportRepository::list`/`count`.
- Official GS1 Digital Link conformance vectors (`dpp-digital-link`).
- Independent Ed25519 cross-verification and DID Core structural tests
  (`dpp-crypto`).
- `validate_strict` fail-closed contract and per-sector schema conformance
  tests (`dpp-domain`).

### Fixed
- ESPR/repairability citations aligned across README, docs, and examples.

## [0.1.1] - 2026-06-27

### Added
- Compliance pipeline extended to carry a compliance result end-to-end.

## [0.1.0] - 2026-06-25

Initial publication to crates.io: `dpp-domain`, `dpp-crypto`,
`dpp-digital-link`, `dpp-registry`, `dpp-rules`, `dpp-calc`,
`dpp-plugin-traits`, `dpp-plugin-sdk`. Built out from an empty workspace in
one pass, crate by crate:

- `dpp-domain`: domain types, `Passport`, sector data, catalog, port traits,
  the transfer-of-responsibility model, versioned schema registry.
- `dpp-crypto`: Ed25519 key management, JWS sign/verify, `did:web` builder,
  verifiable credentials, access policy engine.
- `dpp-digital-link`: GS1 Digital Link parser, link-type negotiation,
  JSON-LD context, AAS mapping.
- `dpp-rules`: ESPR cross-field regulatory rules (`no_std`).
- `dpp-registry`: EU Central Registry interface bridge.
- `dpp-plugin-traits` / `dpp-plugin-sdk`: the Wasm sector-plugin ABI contract
  and guest-side SDK.
- `dpp-tests`: cross-crate integration tests (textile end-to-end, transfer
  of responsibility, access tiers, schema conformity).

Alongside the publishable crates: Wasm sector plugins (`plugins/sector-*`,
released as `.wasm` artefacts, not to crates.io), Criterion micro-benchmarks
(`dpp-benches`, pinned outside lockstep versioning), and the initial
architecture/governance/regulatory documentation set. Internal
workspace-path dependencies versioned and a `LICENSE` bundled per crate
ahead of the crates.io publish.
