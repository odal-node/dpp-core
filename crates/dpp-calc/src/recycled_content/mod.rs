//! Art. 8 minimum recycled content — Regulation (EU) 2023/1542.
//!
//! The determination an operator most often needs from a battery passport today,
//! and the first one in this crate that a real passport can actually feed.
//!
//! ## What this adds over `dpp-rules`
//!
//! `dpp_rules::batteries::recycled_content` already holds the thresholds, the
//! phase dates and the comparison, and the Wasm product group plugin already calls
//! them. What it cannot produce — because it is `no_std` and zero-dependency by
//! contract — is the part a notified body reads:
//!
//! - a **ruleset id and version**, so a finding can say which rule it applied;
//! - an **[`Effectivity`](crate::ruleset::Effectivity)**, so being outside a
//!   phase is reported as being outside a phase rather than as a shortfall;
//! - a **[`RegulatoryBasis`](crate::ruleset::RegulatoryBasis)**, so the citation
//!   travels with the finding instead of living in a source comment;
//! - a **[`CalculationReceipt`](crate::receipt::CalculationReceipt)**, so the
//!   determination can be re-run and checked.
//!
//! Before this module, those four were available only to a two-term CO₂e sum
//! that nothing calls, while the Art. 8 finding — which the battery plugin emits
//! on real data — carried none of them.
//!
//! ## Two phases, and a scope difference that is not a date
//!
//! Art. 8(2) binds industrial (> 2 kWh), electric-vehicle and SLI batteries from
//! 18 August 2031. Art. 8(3) raises the same shares from 18 August 2036 **and is
//! the first to reach LMT batteries at all**. An LMT battery placed on the market
//! in 2032 is therefore not short of anything — it is outside Art. 8(2)'s scope,
//! which is a different answer from meeting it and a different answer again from
//! failing it.
//!
//! That distinction is carried by
//! [`resolve_recycled_content`](crate::ruleset_registry::resolve_recycled_content),
//! which returns an [`Assessability`](crate::assessability::Assessability) — so
//! `NotYetInForce` and `OutOfScope` arrive as themselves rather than as a silent
//! empty result.
//!
//! ## Module layout (five-file methodology convention)
//!
//! - [`parameters`] — [`RecycledContentInputs`], the four declared shares.
//! - [`thresholds`] — [`RecycledContentRuleset`] + the two phase rulesets.
//! - [`calculator`] — [`calculate`], [`RecycledContentResult`], [`MetalShortfall`].
//! - `golden_vectors` — `#[cfg(test)]` regression tests.

pub mod calculator;
pub mod parameters;
pub mod thresholds;

#[cfg(test)]
mod golden_vectors;

pub use calculator::{MetalShortfall, RecycledContentResult, calculate};
pub use parameters::RecycledContentInputs;
pub use thresholds::{Art8Phase1Ruleset, Art8Phase2Ruleset, RecycledContentRuleset};
