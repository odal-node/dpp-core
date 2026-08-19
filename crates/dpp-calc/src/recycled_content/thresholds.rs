//! The two Art. 8 rulesets: effective periods, legal citation, phase dispatch.

use chrono::NaiveDate;
use std::sync::OnceLock;

use dpp_rules::batteries::recycled_content as rules;

use super::calculator::MetalShortfall;
use super::parameters::RecycledContentInputs;
use crate::ruleset::{Effectivity, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

/// Ruleset for an Art. 8 minimum-recycled-share phase.
///
/// Extends [`Ruleset`], so every phase carries its own citation and its own
/// effective period. The period is the point: which phase binds a battery is
/// decided by the date it was placed on the EU market, and
/// [`Effectivity::ensure_active_on`] is the same machinery every other
/// methodology in this crate uses to make that decision.
///
/// The comparison itself is **not** reimplemented here. The thresholds and the
/// check against them live in `dpp-rules`, so the rule has one implementation
/// whether it is reached from a Wasm sector plugin or from this crate; each
/// implementation below only says *which* of them its phase means. What this
/// crate adds is the ruleset identity, the effective period, and the receipt.
pub trait RecycledContentRuleset: Ruleset {
    /// Metals whose declared share falls below this phase's minimums.
    fn shortfalls(&self, inputs: &RecycledContentInputs) -> Vec<MetalShortfall>;
}

fn lift(sf: Vec<rules::RecycledContentShortfall>) -> Vec<MetalShortfall> {
    sf.into_iter()
        .map(|s| MetalShortfall {
            metal: s.material.to_owned(),
            declared_pct: s.declared_pct,
            required_pct: s.required_pct,
        })
        .collect()
}

// ── Art. 8(2) — from 18 August 2031 ──────────────────────────────────────────

static PHASE1_ID: RulesetId = RulesetId("battery-recycled-content-art8-2");
static PHASE1_VERSION: RulesetVersion = RulesetVersion("1.0.0");
static PHASE1_EFFECTIVITY: OnceLock<Effectivity> = OnceLock::new();

static PHASE1_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "EU 2023/1542",
    article: "Art. 8(2) (minimum shares of recycled cobalt, lead, lithium and \
              nickel); Art. 8(1) (declaration per battery model, per year and \
              per manufacturing plant)",
    standard: None,
    technical_study: None,
    source_url: Some("https://eur-lex.europa.eu/eli/reg/2023/1542/oj"),
    // Art. 8(3) raises the same shares for batteries placed on the market from
    // 18 Aug 2036, so this phase governs a closed window rather than running on.
    superseded_by: Some("battery-recycled-content-art8-3"),
};

/// Art. 8(2): the shares binding industrial (> 2 kWh), electric-vehicle and SLI
/// batteries placed on the EU market from 18 August 2031.
///
/// **LMT batteries are outside this ruleset.** Art. 8(2) does not name them;
/// only Art. 8(3) does. That is a scope difference, not a date difference, so it
/// is expressed by the registry's row table rather than by this effectivity.
pub struct Art8Phase1Ruleset;

impl Ruleset for Art8Phase1Ruleset {
    fn id(&self) -> &RulesetId {
        &PHASE1_ID
    }

    fn version(&self) -> &RulesetVersion {
        &PHASE1_VERSION
    }

    fn effectivity(&self) -> &Effectivity {
        // Closed rather than open-ended: a battery placed on the market from
        // 18 Aug 2036 is governed by Art. 8(3)'s higher shares, so leaving this
        // open would make both phases active on the same date and the answer
        // depend on the order of the registry's rows.
        PHASE1_EFFECTIVITY.get_or_init(|| {
            let last_day = art8_phase2_from()
                .pred_opt()
                .expect("18 Aug 2036 has a preceding day");
            Effectivity::closed(art8_phase1_from(), last_day)
        })
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &PHASE1_BASIS
    }
}

impl RecycledContentRuleset for Art8Phase1Ruleset {
    fn shortfalls(&self, inputs: &RecycledContentInputs) -> Vec<MetalShortfall> {
        lift(rules::art8_shortfalls_2031(&inputs.into()))
    }
}

// ── Art. 8(3) — from 18 August 2036 ──────────────────────────────────────────

static PHASE2_ID: RulesetId = RulesetId("battery-recycled-content-art8-3");
static PHASE2_VERSION: RulesetVersion = RulesetVersion("1.0.0");
static PHASE2_EFFECTIVITY: OnceLock<Effectivity> = OnceLock::new();

static PHASE2_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "EU 2023/1542",
    article: "Art. 8(3) (raised minimum shares, and the first to reach LMT \
              batteries); Art. 8(1) (declaration per battery model, per year \
              and per manufacturing plant)",
    standard: None,
    technical_study: None,
    source_url: Some("https://eur-lex.europa.eu/eli/reg/2023/1542/oj"),
    superseded_by: None,
};

/// Art. 8(3): the raised shares binding industrial (> 2 kWh), electric-vehicle,
/// SLI **and LMT** batteries placed on the EU market from 18 August 2036.
pub struct Art8Phase2Ruleset;

impl Ruleset for Art8Phase2Ruleset {
    fn id(&self) -> &RulesetId {
        &PHASE2_ID
    }

    fn version(&self) -> &RulesetVersion {
        &PHASE2_VERSION
    }

    fn effectivity(&self) -> &Effectivity {
        PHASE2_EFFECTIVITY.get_or_init(|| Effectivity::open(art8_phase2_from()))
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &PHASE2_BASIS
    }
}

impl RecycledContentRuleset for Art8Phase2Ruleset {
    fn shortfalls(&self, inputs: &RecycledContentInputs) -> Vec<MetalShortfall> {
        lift(rules::art8_shortfalls_2036(&inputs.into()))
    }
}

// ── The phase dates, read from where they are defined ────────────────────────

fn to_naive(d: dpp_rules::common::date::CalendarDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year, u32::from(d.month), u32::from(d.day))
        .expect("an Art. 8 phase date is a real calendar date")
}

/// 18 August 2031 — Art. 8(2). Converted from `dpp-rules`' `CalendarDate`, which
/// is `no_std` and so does not use `chrono`.
pub(crate) fn art8_phase1_from() -> NaiveDate {
    to_naive(rules::ART8_PHASE1_FROM)
}

/// 18 August 2036 — Art. 8(3).
pub(crate) fn art8_phase2_from() -> NaiveDate {
    to_naive(rules::ART8_PHASE2_FROM)
}
