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
//! # Effective dates
//!
//! How a governing ruleset is selected, why `Pending` has no date, and the
//! conditional `max(floor, entry-into-force + N months)` arithmetic that is
//! deliberately not built yet:
//! see `docs/architecture/EFFECTIVE-DATES.md`.
//!
//! ## Pending delegated act (stub)
//!
//! Use `Effectivity::pending(empowerment, adoption_deadline)` and
//! `regulatory_basis.regulation = "pending — {PEFCR title}"`.
//!
//! There is **no date sentinel**. A pending ruleset has no application date, so
//! it resolves for no date at all and `ensure_active_on` returns
//! `CalcError::RulesetUndetermined` naming the instrument being waited on. The
//! type stays compile-visible for future wiring. A far-future `from` date would
//! assert something the regulation does not say — EU staged obligations are
//! written as *"from &lt;date&gt; or N months after entry into force, whichever
//! is the latest"*, so until the act lands the date is unknown, not distant.
//!
//! ## Superseded ruleset
//!
//! - Set the ruleset's `Effectivity` to `closed(from, last_valid_day)`.
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
pub use kernel::{assessability, clock, error, factor, receipt, ruleset};

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
