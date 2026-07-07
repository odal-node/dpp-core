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
