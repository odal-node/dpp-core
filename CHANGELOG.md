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

## [0.11.0] - 2026-07-25

### Breaking

- **`AccessTier` is replaced by `Audience` + `Disclosure`.** Reg. (EU) 2023/1542
  Art. 77(2) assigns three audiences to four Annex XIII data sets, and it is not
  an ordering: conformity test reports (point 3) go to authorities only, and
  individual-battery data (point 4) to legitimate-interest holders only, so
  neither audience contains the other. Any `>=` comparison necessarily either
  hands authorities data the regulation withholds or hides point-2 data from
  someone entitled to it. *Migration:* `filter_by_access_tier` →
  `filter_by_audience`; `SectorAccessPolicy::{field_tiers, default_tier,
  tier_for_field}` → `{field_disclosure, default_disclosure,
  disclosure_for_field}`; `SectorDescriptor::access_tiers` → `disclosure`, and
  the manifest key `accessTiers` → `disclosure` with values `professional` →
  `restricted`, `confidential` → `conformity`; `CredentialRole::access_tier()` →
  `audience()`; `TrustedIssuerRegistry::is_trusted_for_tier` →
  `is_trusted_for_audience`; `Passport::redact` and `redact_sector_data` take an
  `Audience`.
- **Calculators select their ruleset by governing-law date.** `co2e::calculate`
  and `repairability::calculate` take an `AssessmentClock { law_in_force_on,
  computed_at }`; the wall-clock overloads and `calculate_asof` are removed, as
  is `Ruleset::ensure_active_today`. Staged EU obligations attach at a regulated
  triggering event, so a battery placed on the market in 2030 must not acquire
  the 18 Aug 2031 minimums by being audited in 2033. *Migration:* build a clock
  with `AssessmentClock::placed_on(date)` from the product's own record — there
  is deliberately no `now()`.
- **`EffectiveDateBound` is replaced by `Effectivity`**, which can state
  `Pending { empowerment, adoption_deadline }`. The `2100-01-01` sentinel is
  gone: it asserted a calendar date for an act that does not exist.
  `Ruleset::effective_dates()` → `effectivity()`.
- **`resolve_repairability` returns `Assessability<T>`** instead of `Option`,
  separating `NotYetInForce` / `Undetermined` / `Expired` / `OutOfScope`. None of
  those is non-compliance, and `Option` could not tell them apart.
- **`CalculationReceipt::new` / `for_ruleset` take an `AssessmentClock`** and the
  receipt gains `assessed_as_of`. Without it a receipt names the ruleset it cited
  but not whether that was the right one to cite.
- **`CarbonFootprintClass` is a validated newtype, not an `A`–`E` enum.**
  Art. 7(2) defines no class labels and requires the Commission to review the
  number of classes every three years; the previous `#[serde(other)]` variant
  silently flattened any unrecognised label to `Other`, losing it on round trip
  under a qualified seal. *Migration:* `CarbonFootprintClass::new(label)?`.
- **`annex_x_shortfalls_{2031,2036}` → `art8_shortfalls_{2031,2036}`.** Annex X
  is the due-diligence raw-materials list; the recycled-content shares are in
  Art. 8(2)/(3), which cross-reference Annex VIII.
- **`BatteryData` gains fields and boxes two.** New: `placed_on_market_date`,
  `carbon_footprint_class_ruleset_id` / `_version`,
  `recycled_content_reporting_year`, `state_of_health`, `expected_lifetime`. The
  two Annex VII measurement blocks are boxed — they are large, optional and
  rarely populated, and inline they pushed `SectorData` past clippy's
  `large_enum_variant` threshold.

### Added

- **Annex VII state of health and expected lifetime** (battery schema v2.2.0 and
  v2.4.0). Both parts are modelled as sum types because each is two disjoint
  lists, and the optionality follows the annex's own wording — items qualified
  "where possible" are `Option`, the rest are required. Part B deliberately
  excludes EV batteries: Part A names them, Part B does not.
- **Art. 8(1) declaration duty** (`art8_declaration_duty_for`), separate from the
  Art. 8(2)/(3) minimums. Its date is conditional on an unadopted delegated act,
  so it answers `NotYetDue` below the floor (certain — the real date can only be
  the floor or later), `Undetermined` on or after it. Schema v2.3.0 adds
  `recycledContentReportingYear`: Art. 8(1) requires the shares per model, per
  year and per plant, so a bare percentage is not the declaration.
- **`docs/architecture/EFFECTIVE-DATES.md`** — the date model, and a
  specification for the deferred `max(floor, entry-into-force + N months)`
  arithmetic, including the rule that entry into force is read from the Official
  Journal and never inferred.
- **Art. 77(3) and Art. 78(d) conformance positions** in
  `docs/regulatory/CONFORMITY.md`, both marked with what is verified and what is
  not.
- **`just test-plugins`, wired into `just check`.** The sector plugins are
  excluded from the workspace, so the gate never ran their tests; it caught two
  real failures the first time it ran.
- **Drift guard on product categories** — a catalog category that is not a legal
  value of its schema enum now fails the build.

### Fixed

- **`lintResult` leaked into the public passport view.** `Passport::redact` and
  `SectorAccessPolicy::passport_default()` each carried their own copy of the
  field classification and had drifted: the policy classified `lintResult` as
  restricted, `redact` never removed it. Both now read
  `PASSPORT_FIELD_DISCLOSURE`.
- **The QR serial disclosed the passport's creation time.** `short_serial` cut
  the GS1 AI 21 serial from the first ten bytes of a UUIDv7 — a millisecond
  timestamp — so serials sorted in creation order and their first twelve hex
  characters decoded to the creation instant. Now derived from the random tail.
  **This changes the QR URL for a given passport;** the resolver is GTIN-keyed,
  so the serial is not a resolution key.
- **Three wrong legal citations**, verified against the Official Journal text:
  Annex X → Art. 8(2)/(3) for recycled content, Art. 10(6) → Art. 10(5) for the
  minimum-values empowerment, and `REGULATORY.md` had both recycled-content phase
  dates as 1 January rather than 18 August, excluded SLI from Phase 1, and placed
  LMT in Phase 1 instead of Phase 2.
- **`Regime::Other` was unreachable.** An externally-tagged `Other(String)`
  rendered as `{"other": "…"}` and an unrecognised value failed the whole
  deserialise, so a manifest naming a new instrument broke catalog loading.
- **State of health was served publicly.** It was unclassified and fell through
  to the public default; Annex XIII point 4(b) restricts it to persons with a
  legitimate interest.
- **Repairability index classified an unrounded value.** Annex IV point 5.4
  rounds to two decimals before classification against the Annex II Table 4
  boundaries.
- **Reg. (EU) 2025/1561 recorded** in the battery `legalBasis`, read from the OJ:
  it amends Art. 48 only, moving the due-diligence trigger to 18 Aug 2027 and the
  guidelines deadline to 26 Jul 2026. The widely-reported "reduced scope" is not
  in the instrument.


### Breaking

- **`DppSectorPlugin::generate_passport` now takes `PluginInput` by value**
  instead of `&PluginInput`, so a pass-through implementation returns its
  input directly instead of cloning it. `meta()` and `capabilities()` now
  have default implementations built from two new required methods,
  `plugin_identity()` and `schema_version_range()` — a plugin supplies its
  identity fields and supported schema range once, instead of hand-assembling
  the full `PluginMeta`/`PluginCapabilities` structs. *Migration:* implement
  `plugin_identity()` and `schema_version_range()`; drop a hand-rolled
  `meta()`/`capabilities()` unless it genuinely needs non-default values.
- **`RulesetId`/`RulesetVersion` now wrap `&'static str`** instead of
  `String`, and no longer derive `Deserialize` — every concrete `Ruleset`
  impl in this crate constructs them from a compile-time literal, never from
  external input. *Migration:* replace `RulesetId("...".to_owned())` with
  `RulesetId("...")`; a caller that was deserializing a `RulesetId` from
  untrusted input was never a supported use and has no replacement.
- **Sector GTIN fields migrated from `String` to the validated `Gtin`
  newtype** across aluminium, construction, detergent, electronics,
  furniture, steel, textile, toy, and tyre sector data. *Migration:* build
  with `Gtin::parse(&s)?` instead of assigning a raw string; read with
  `.as_str()` or `Display` where a `&str`/`String` is needed.
- **Country-of-origin field unified across sectors.** Aluminium,
  construction, detergent, furniture, steel, textile, and toy sector data
  previously used sector-specific names (`countryOfManufacture`,
  `countryOfProduction`, `countryOfManufacturing`); all now use
  `countryOfOrigin` (`country_of_origin` in Rust). Each affected sector's
  JSON Schema gained a new minor version reflecting the renamed field.
  *Migration:* update field access and any stored/transmitted JSON using the
  old per-sector names.
- **`MaterialEntry::origin_country` renamed to `country_of_origin`** for
  consistency with the sector-level rename above.

### Added

- **`dpp_calc::co2e::calculate_asof`** — computes a CO2e footprint against
  the ruleset version effective on a caller-supplied date, rather than always
  the latest registered version. Needed to reproduce a historical calculation
  (e.g. re-verifying an evidence dossier) under the ruleset that was actually
  in force when it was originally computed.
- A libFuzzer target round-tripping the plugin ABI envelope
  (`AbiResult`/`PluginCapabilities`) through encode/decode, catching panics
  on malformed Wasm-boundary input. CI-only; not part of `just check`.

### Changed

- **`dpp-registry::RegistryStatusCode` is now re-exported from the crate
  root** — previously reachable only via its full internal module path.
- Sector plugins (`plugins/sector-*`) consolidated into a shared Cargo
  workspace instead of 10 independently-versioned crates, and their
  duplicated codec-dispatch, country-code table, and `threshold_status`
  logic now live in `dpp-plugin-sdk`, shared instead of copy-pasted per
  plugin.
- `Passport::validate()` now returns structured, JSON-Pointer-addressed
  field errors (`ValidationErrors`) instead of a flat error list, and
  consolidates several previously-scattered validation helpers.
- Numerous internal-only dedup refactors with no public API change: shared
  test fixtures across `dpp-tests`/benches/`dpp-crypto`/`dpp-plugin-traits`,
  shared numeric-tolerance helpers in `dpp-rules`, shared receipt/threshold
  helpers in `dpp-calc`, and `dpp-crypto` keystore cleanup avoiding an
  unnecessary decrypt-for-pubkey round trip.

### Fixed

- Two stale architecture claims in `dpp-digital-link`'s and
  `dpp-plugin-traits`' docs corrected.

### Performance

- **`dpp-digital-link`'s JSON-LD passport context is now cached** instead of
  rebuilt on every call.
- `BatteryData`'s AAS submodel builder now calls `BatteryChemistry::wire_str()`
  directly instead of the generic JSON-round-trip `enum_wire_str()` helper.
- `Gtin`/`Gln` check-digit validation no longer heap-allocates a `Vec` per
  call — uses a fixed stack buffer instead.

## [0.10.0] - 2026-07-20

### Added

- **`dpp_rules::canonical::content_hash`** — the single canonical JCS
  (RFC 8785) content hasher, shared by ruleset-bundle verification and by
  downstream evidence consumers that previously re-implemented it. Fallible by
  design: RFC 8785 rejects non-finite floats, and a hasher fed untrusted input
  must be able to return an error rather than abort the process. Behind the
  `bundle` feature, which supplies the JCS and SHA-256 dependencies.

### Breaking

- **`lintResult` is no longer served at the `Public` access tier.**
  `SectorAccessPolicy::passport_default()` now maps it to
  `AccessTier::Professional`. The lint result is deliberately re-computable
  after publish and every re-run restamps `assessedAt`, so serving it `Public`
  placed a guaranteed-to-change field inside the payload the public signature
  is computed over — a served body that stops verifying against its own proof
  for reasons that are not tampering. *Migration:* consumers that read
  `lintResult` from an unauthenticated public view must request the
  Professional tier; it is operator- and auditor-facing quality data, not
  consumer-facing content.

### Changed

- **`patch_fields` now rejects the lineage edges.** `parentPassportRef` and
  `componentRefs` joined `PROTECTED_PATCH_FIELDS`. Both are create-time by
  construction and sit inside the signed public view: a second-life passport is
  issued as a *new* record, and changing a published bill of materials is a new
  passport version (`supersedesId`), not an in-place edit.
- **`dpp_rules::bundle::verify::content_hash` now delegates** to
  `canonical::content_hash`, mapping its error into `RulesetError::Malformed`.
  Signature and behaviour are unchanged; a consumer outside the ruleset channel
  should call the canonical function directly rather than adopt `RulesetError`.

### Fixed

- **The `bundle` module was never compiled or tested by this workspace's own
  gate.** No crate enables `dpp-rules`' `bundle` feature, so `just check` and CI
  silently skipped the signed-ruleset format and its fail-closed verification
  entirely — two tests in it had been failing on `main` since before 0.9.0.
  `--all-features` is now passed to test, lint and doc in both the justfile and
  CI, and the two tests were repaired: their premise (that `serde_json` coerces
  an overflowing float literal to infinity) no longer holds, so they became
  tripwires on the upstream guarantee that keeps the hasher's error path
  unreachable.

### Documentation

- **`docs/architecture/PRODUCT-LINEAGE.md`** — design proposal for the
  bill-of-materials and second-life edges, recording the requirements pass the
  initial cut shipped without. Headline finding: Battery Regulation (EU)
  2023/1542 Art. 77 requires a second-life passport linked to "the battery
  passport **or passports** of the original battery **or batteries**", which
  `parent_passport_ref: Option<PassportRef>` cannot express. Not implemented;
  phases 2–3 are breaking and scheduled for a later minor.
- **EU registry status corrected, and its rules pinned to the OJ text.** The
  registry became operational on 20 July 2026 under Commission Implementing
  Regulation (EU) 2026/1778. `COMPLIANCE.md` now records, with article
  citations, who may register and on what credential (Arts. 4–5, 8(1)), that a
  third party may register on an authorised operator's behalf while the operator
  remains fully responsible (Art. 19(4)–(5)), the registry's structure
  (Art. 3), the granularity and identifier-linking rules (Art. 8(1), (3)–(5)),
  and the automated checks applied on submission (Art. 8(7)). The preparatory
  `dpp-registry` types are recorded as *known to diverge* from it: no commodity
  code (stored for customs "release for free circulation" per Art. 3(e) and
  validated where relevant per Art. 8(7)(d)), no granularity or
  identifier-linking concept, and a bearer-token auth assumption where the
  specification uses eIDAS verified-operator identity.
- **`SealPort` records what "qualified" requires**, verified against the OJ text
  of Regulation (EU) No 910/2014: a qualified electronic seal is the conjunction
  of an advanced seal (Art. 36), a qualified signature creation device
  (Art. 3(32)) and a qualified certificate (Art. 3(30), Annex III). An adapter
  producing an advanced seal over a qualified certificate does not satisfy it.
- **Transfer-of-responsibility article pin** in `COMPLIANCE.md`, reconciled
  against the operative text: no article establishes a transfer mechanism;
  Art. 11(1)(e) carries the continuity duty, while Art. 9(1)'s data-accuracy
  duty — cited elsewhere in the repo as part of the transfer basis — stands.

## [0.9.0] - 2026-07-17

### Added

- **`dpp-digital-link::short_serial`** — derives a GS1-conformant AI 21 serial
  from a passport UUID: the first 10 bytes encoded as lowercase hex, exactly
  20 characters, drawn only from `[0-9a-f]` (URL-safe and within the GS1
  encodable set). Non-sequential, so a public carrier leaks no production
  volume. This is the intended way to put a passport id on a physical carrier
  now that AI value lengths are enforced (see Breaking).
- **Typed errors for the new validation** (additive variants):
  `DigitalLinkError::ValueTooLong` (an AI value exceeds its GS1 length cap);
  `dpp-calc` kernel errors for a ruleset whose effective period has not started
  and for a computation that overflows to a non-finite value; catalog
  descriptor errors for a `current_schema_version` that is not valid semver or
  not listed in `schema_versions`.

### Breaking

- **GS1 AI value lengths are enforced at parse time.** `DigitalLink` parsing
  rejects an AI value exceeding its GS1 General Specifications cap with
  `DigitalLinkError::ValueTooLong { code, max_len, actual }` — notably a serial
  (AI 21) longer than 20 characters, which previous releases accepted. A raw
  36-character passport UUID can no longer be carried in AI 21. *Migration:*
  derive the carrier serial with the new `short_serial`, or supply your own
  GS1-conformant value. `build_qr_url` documents the same contract: GTIN
  (AI 01), optional batch/lot (AI 10), serial (AI 21) — values percent-encoded,
  serial GS1-conformant.

### Fixed

- **Non-finite values can no longer masquerade as results.** A plugin metric
  inserted directly into `PluginResult.metrics` (bypassing the `with_metric`
  guard) fails serialisation on `NaN`/`Infinity` instead of silently becoming
  JSON `null`, and the ABI envelope returns `AbiResult::Error` rather than a
  spurious success. `dpp-calc` refuses a computation that overflows to a
  non-finite value — a legally cited figure must never silently become
  Infinity.
- `dpp-calc` rejects a ruleset whose effective period has not started (e.g. a
  pending delegated act using the `2100-01-01` sentinel) instead of computing
  against it.
- Catalog descriptor validation: `current_schema_version` must be valid semver
  and appear in `schema_versions`.
- Input-validation and error-handling hardening across the keystore (migration
  and store), registry identifier parsing, passport validation functions,
  rules bundle verification, JSON-LD context handling, and the in-repo Wasm
  sector plugins.

### Documentation

- `docs/architecture/IDENTITY.md` §6 rewritten to the shipped carrier design:
  the QR encodes a GS1 Digital Link built from verified passport fields on the
  deployment's configured resolver base (replaces the description of a
  proprietary-path scheme with a `sig` hint that was never implemented).
- `README.md` proof-bound wording made precise (what is discarded vs. what is
  retained and served under tiered access) and the standards sentence corrected
  (six of the eight JTC 24 ENs published 27 May 2026; EN 18239/18246 pending).
- `docs/regulatory/CONFORMITY.md` standards currency updated.

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
