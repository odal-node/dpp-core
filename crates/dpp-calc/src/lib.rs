//! EU-methodology compliance calculators for Odal Node.
//!
//! Pure, stateless calculation of regulatory metrics from typed inputs. No I/O,
//! no infrastructure — every function is deterministic and side-effect free.
//!
//! **Placement:** these compute *EU methodology* (cradle-to-gate CO₂e, battery
//! CFB stub) plus a non-regulatory repairability heuristic, so they change when a
//! regulation changes and therefore live in `dpp-core` under Apache-2.0 — not in
//! the platform.
//! The licensing split: the methodology is open; licensed LCI datasets are
//! never bundled here and are injected at runtime via [`factor::FactorProvider`].
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │  dpp-calc (Apache-2.0, open)                                         │
//! │                                                                      │
//! │  co2e::calculate()          — cradle-to-gate, operator-supplied EFs  │
//! │  repairability::calculate() — non-regulatory heuristic → A–E band    │
//! │  co2e::cfb::calculate_cfb() — STUB → CalcError::NotImplemented       │
//! │                                                                      │
//! │  ruleset_registry           — date-based ruleset resolution          │
//! │  Ruleset / RegulatoryBasis  — legal citation in every ruleset        │
//! │  FactorProvider             — runtime injection point for LCI data   │
//! │  CalculationReceipt         — proof-of-calculation envelope          │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Every calculator emits a [`receipt::CalculationReceipt`] that records the
//! input hash, ruleset id + version, factor dataset version and table hash —
//! ready to be stored in the proof-bound store for notified-body audit.
//!
//! # Adding a new sector calculator
//!
//! ## New methodology (algorithm not yet in dpp-calc)
//!
//! 1. Add `src/{methodology}/` with four files (see `co2e/` or `repairability/`):
//!    - `mod.rs`        — `pub fn calculate(inputs, ruleset) -> Result<...>` + output types
//!    - `parameters.rs` — typed input struct (derives `Serialize` for receipt hashing)
//!    - `thresholds.rs` — `pub trait {Methodology}Ruleset: Ruleset { ... }` + concrete impls
//!    - `golden_vectors.rs` — `#[cfg(test)]` regression tests
//! 2. Every `impl RepairabilityRuleset / Co2eRuleset / CfbRuleset` must also
//!    `impl Ruleset` and fill `regulatory_basis()` with the EU citation.
//! 3. Add a `resolve_{methodology}()` function to `ruleset_registry/resolve.rs`
//!    and a `&NewRuleset` row to `all_rulesets()`.
//! 4. Write golden vectors, including the `all_concrete_rulesets_have_non_empty_regulatory_basis` pattern.
//! 5. Register the module in `lib.rs`.
//!
//! ## New product category on an existing methodology
//!
//! 1. Add `impl Ruleset + impl {Methodology}Ruleset` for the new struct in `thresholds.rs`.
//!    Fill `regulatory_basis` from the product-specific delegated act.
//! 2. Add a row to the resolver table in `ruleset_registry/resolve.rs` and to
//!    `all_rulesets()` (one-liner each).
//! 3. Add golden vectors. Run `cargo test -p dpp-calc`.
//!
//! ## Pending delegated act (stub)
//!
//! Use `EffectiveDateBound::open(NaiveDate(2100, 1, 1))` as the sentinel and
//! `regulatory_basis.regulation = "pending — {PEFCR title}"`.
//! The effective-date guard blocks runtime use; `resolve_*` returns `None` for
//! all real dates. Keeps the type compile-visible for future plugin wiring.
//!
//! ## Superseded ruleset
//!
//! - Set `EffectiveDateBound.until` to the last valid day.
//! - Set `regulatory_basis.superseded_by` to the new ruleset's ID string.
//! - Keep the row in `ruleset_registry` — receipts reference rulesets by
//!   ID + version, so removing a row makes old receipts unverifiable.

#![forbid(unsafe_code)]

// Methodology-agnostic spine. Kept private and re-exported below so the internal
// grouping never leaks: external callers keep using `dpp_calc::error`,
// `dpp_calc::factor`, `dpp_calc::receipt`, and `dpp_calc::ruleset`.
mod kernel;

pub mod co2e;
pub mod repairability;
pub mod repairability_index;
pub mod ruleset_registry;

// ── Stable public paths for the spine ────────────────────────────────────────
pub use kernel::{error, factor, receipt, ruleset};
