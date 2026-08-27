//! Battery recycled content validation — EU Regulation 2023/1542, Art. 8.
//!
//! Art. 8(2) and 8(3) set minimum recycled content shares for four metals, in
//! the regulation text itself. Both paragraphs require the **Annex VIII**
//! technical documentation to demonstrate the share — Annex VIII is the only
//! annex Art. 8 cross-references.
//!
//! Phase 1 (from 18 Aug 2031) covers **EV batteries, SLI batteries, and
//! industrial batteries with a capacity > 2 kWh** (excluding those with
//! exclusively external storage). **LMT batteries** join only in Phase 2 (from
//! **18 Aug 2036**), at the higher targets. Portable batteries are out of scope.
//!
//! The targets are **finalized law** — they are in the regulation text itself,
//! not in a pending delegated act. However, neither phase is yet in force, so
//! the battery plugin reports an overall status of `NotAssessed` and surfaces
//! any shortfall as an advisory warning rather than a violation. These constants
//! are the single source of truth it will check against once enforcement begins.
//!
//! ## The measurement basis differs by metal
//!
//! Art. 8(2)/(3) measure cobalt, lithium and nickel as the share recovered from
//! waste that is present **in active materials**; lead is measured as the share
//! present **in the battery**. The percentages below therefore do not share a
//! denominator, and a caller must not sum or average them across metals.
//!
//! ## Two duties, not one
//!
//! Art. 8(1) requires documentation of the *actual* shares, with no minimum,
//! from **18 Aug 2028** (industrial > 2 kWh, EV, SLI) and **18 Aug 2033** (LMT)
//! — or 24 months after the Art. 8(1) methodology delegated act enters into
//! force, whichever is later. Annex XIII point 1(e) makes that documentation
//! publicly accessible passport content. See [`art8_declaration_duty_for`].
//!
//! Art. 8(2) and 8(3) then impose *minimum* shares from 18 Aug 2031 and
//! 18 Aug 2036 — fixed dates, unconditional. See [`art8_phase_for`].
//!
//! The two ladders are independent: a battery can owe the declaration without
//! yet owing a minimum.
//!
//! ## Which phase binds is a function of the placing-on-market date
//!
//! [`art8_category_for`] maps a declared battery type and capacity onto an
//! [`Art8Category`]; [`art8_phase_for`] then picks the governing phase from the
//! date the battery was placed on the EU market. Assessment date is never an
//! input — Art. 8 attaches its duties at placing on the market, so deriving the
//! phase from "today" would report batteries lawfully placed before a phase
//! began as short of it from the moment that phase starts.
//!
//! The battery plugin calls both, and reports a missing or unparseable
//! `placedOnMarketDate` as its own finding rather than assuming one.
//!
//! ## Phase 1 — EV + SLI + industrial > 2 kWh, from **18 Aug 2031** (Art. 8(2))
//! | Material | Minimum % | Basis |
//! |----------|-----------|-------|
//! | Cobalt   |      16 % | active materials |
//! | Lead     |      85 % | the battery |
//! | Lithium  |       6 % | active materials |
//! | Nickel   |       6 % | active materials |
//!
//! ## Phase 2 — Phase 1 categories + **LMT**, from **18 Aug 2036** (Art. 8(3))
//! | Material | Minimum % | Basis |
//! |----------|-----------|-------|
//! | Cobalt   |      26 % | active materials |
//! | Lead     |      85 % | the battery |
//! | Lithium  |      12 % | active materials |
//! | Nickel   |      15 % | active materials |

use alloc::vec::Vec;

use crate::common::date::CalendarDate;

// ✅ COMPLIANCE-PIN: EU 2023/1542, Art. 8(2) and 8(3) (OJ L 191, 28.7.2023, p. 33)
// Verified verbatim against the Official Journal text on 2026-07-25. Percentages,
// dates and category scope were read directly from Art. 8(2) and 8(3); the prior
// 🟠 residual (verbatim OJ confirmation) is now closed.
// Phase-1 date: 18 Aug 2031. Phase-2 date: 18 Aug 2036.
// Category scope: Phase 1 = industrial > 2 kWh (excl. exclusively-external-storage)
// + EV + SLI. Phase 2 adds LMT. SLI is **in** Phase-1 scope.
// Cross-reference is **Annex VIII** (technical documentation), named explicitly by
// both paragraphs. A prior pin here cited "Annex X"; that annex is "LIST OF RAW
// MATERIALS AND RISK CATEGORIES" (due diligence, Arts. 48–53) and is unrelated to
// recycled content. Corrected 2026-07-25.

// ── Phase 1 constants — industrial > 2 kWh + EV + SLI, from 18 Aug 2031 ──────

/// Minimum cobalt recycled content — Art. 8(2), from 18 Aug 2031.
pub const COBALT_RECYCLED_PCT_2031: f64 = 16.0;
/// Minimum lead recycled content — Art. 8(2), from 18 Aug 2031.
pub const LEAD_RECYCLED_PCT_2031: f64 = 85.0;
/// Minimum lithium recycled content — Art. 8(2), from 18 Aug 2031.
pub const LITHIUM_RECYCLED_PCT_2031: f64 = 6.0;
/// Minimum nickel recycled content — Art. 8(2), from 18 Aug 2031.
pub const NICKEL_RECYCLED_PCT_2031: f64 = 6.0;

// ── Phase 2 constants — Phase 1 categories + LMT, from 18 Aug 2036 ───────────

/// Minimum cobalt recycled content — Art. 8(3), from 18 Aug 2036.
pub const COBALT_RECYCLED_PCT_2036: f64 = 26.0;
/// Minimum lead recycled content — Art. 8(3), from 18 Aug 2036.
pub const LEAD_RECYCLED_PCT_2036: f64 = 85.0;
/// Minimum lithium recycled content — Art. 8(3), from 18 Aug 2036.
pub const LITHIUM_RECYCLED_PCT_2036: f64 = 12.0;
/// Minimum nickel recycled content — Art. 8(3), from 18 Aug 2036.
pub const NICKEL_RECYCLED_PCT_2036: f64 = 15.0;

// ── Which phase binds ─────────────────────────────────────────────────────────

/// First day on which the Art. 8(2) minimum shares bind a battery placed on the
/// EU market — 18 August 2031.
pub const ART8_PHASE1_FROM: CalendarDate = CalendarDate::new(2031, 8, 18);

/// First day on which the Art. 8(3) minimum shares bind a battery placed on the
/// EU market — 18 August 2036.
pub const ART8_PHASE2_FROM: CalendarDate = CalendarDate::new(2036, 8, 18);

/// The battery categories Art. 8(2) and 8(3) treat differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art8Category {
    /// Industrial batteries > 2 kWh (excluding those with exclusively external
    /// storage), electric-vehicle batteries, and SLI batteries. Named by both
    /// Art. 8(2) and Art. 8(3).
    IndustrialEvSli,
    /// LMT batteries. Named only by Art. 8(3) — outside Phase 1 entirely.
    Lmt,
    /// Portable batteries, and anything else Art. 8 does not reach.
    NotCovered,
}

/// Which Art. 8 minimum-share phase binds a battery.
///
/// Four outcomes, deliberately not collapsed into `Option`: "Art. 8 does not
/// reach this battery" and "Art. 8 does not reach this battery *yet*" are
/// different answers, and reporting either as a shortfall misstates an
/// operator's legal position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art8Phase {
    /// The minimum shares never bind this category.
    NotCovered,
    /// In scope, but placed on the market before the phase began.
    NotYetBinding,
    /// Art. 8(4) disapplies Art. 8(1)–(3) to this battery: it went through a
    /// second-life operation having already been on the market.
    ///
    /// Distinct from [`Art8Phase::NotCovered`], which says the category is
    /// outside Art. 8 altogether. Here the category *is* covered and the battery
    /// is individually excused, which is a different sentence to put in front of
    /// an operator.
    ExemptSecondLife,
    /// Art. 8(2) minimums apply — the `*_2031` constants.
    Phase1,
    /// Art. 8(3) minimums apply — the `*_2036` constants.
    Phase2,
}

/// Whether Art. 8(4)'s second-life carve-out reaches a battery.
///
/// # Art. 8(4) is not Art. 10(4), and the difference is the condition
///
/// Both articles excuse second-life batteries, and it is tempting to model them
/// together. They test different things:
///
/// - **Art. 8(4)** — the operations, *"if the batteries had already been placed
///   on the market or put into service **before undergoing such operations**"*.
///   Prior placement relative to the *operation*.
/// - **Art. 10(4)** — the operations, where the operator demonstrates placement
///   *"before the dates on which those obligations become applicable"*. Prior
///   placement relative to the *obligation dates*.
///
/// So Art. 8(4) is the broader of the two: a battery placed on the market in
/// 2033 and remanufactured in 2035 is exempt from Art. 8, while the equivalent
/// battery is *not* exempt from Art. 10 because it was placed after those duties
/// applied. Collapsing them would have made one of the two wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art8SecondLife {
    /// An original battery, or one whose history is not established.
    ///
    /// The default for an unknown history, deliberately. Art. 8(4) is an
    /// exemption the operator must be able to demonstrate, and wrongly telling
    /// an operator they are excused is the worse error — a missed duty is
    /// discovered by an authority rather than by us.
    None,
    /// Prepared for re-use, prepared for repurposing, repurposed or
    /// remanufactured, **and** already placed on the market or put into service
    /// before undergoing that operation.
    ///
    /// Both halves are required by the article and the caller asserts both by
    /// choosing this variant.
    PlacedBeforeOperations,
}

/// Select the governing Art. 8 phase from the date the battery was **placed on
/// the EU market** — not the date of assessment.
///
/// Art. 8(2) and 8(3) attach their obligations to batteries placed on the market
/// from their respective dates. A battery lawfully placed on the market in 2030
/// does not acquire the 2031 minimums by being assessed in 2033. Passing
/// "today" here instead of the placing-on-market date produces retroactive
/// non-compliance findings from 18 Aug 2031 onwards.
///
/// `second_life` carries Art. 8(4), which disapplies Art. 8(1)–(3) entirely and
/// is therefore checked before category or date. See [`Art8SecondLife`] for why
/// it is not the same test as Art. 10(4)'s.
#[must_use]
pub fn art8_phase_for(
    category: Art8Category,
    placed_on_market: CalendarDate,
    second_life: Art8SecondLife,
) -> Art8Phase {
    // Art. 8(4) is unconditional on date and category — "paragraphs 1, 2 and 3
    // shall not apply" — so it answers first.
    if second_life == Art8SecondLife::PlacedBeforeOperations {
        return Art8Phase::ExemptSecondLife;
    }
    match category {
        Art8Category::NotCovered => Art8Phase::NotCovered,
        // Art. 8(2) does not name LMT; only Art. 8(3) does.
        Art8Category::Lmt => {
            if placed_on_market >= ART8_PHASE2_FROM {
                Art8Phase::Phase2
            } else {
                Art8Phase::NotYetBinding
            }
        }
        Art8Category::IndustrialEvSli => {
            if placed_on_market >= ART8_PHASE2_FROM {
                Art8Phase::Phase2
            } else if placed_on_market >= ART8_PHASE1_FROM {
                Art8Phase::Phase1
            } else {
                Art8Phase::NotYetBinding
            }
        }
    }
}

/// Map a declared battery type and energy capacity onto the Art. 8 category.
///
/// Matching is case-insensitive. Art. 8(2)/(3) reach industrial batteries only
/// above 2 kWh, so an industrial battery at or below that threshold — or with an
/// undeclared capacity — is [`Art8Category::NotCovered`].
///
/// An unrecognised or absent `battery_type` maps to
/// [`Art8Category::IndustrialEvSli`]. That is the conservative direction: a
/// mislabelled in-scope battery is still assessed rather than silently skipped.
#[must_use]
pub fn art8_category_for(battery_type: &str, capacity_kwh: Option<f64>) -> Art8Category {
    let t = battery_type.trim();
    let eq = |s: &str| t.eq_ignore_ascii_case(s);
    if eq("portable") {
        Art8Category::NotCovered
    } else if eq("lmt") {
        Art8Category::Lmt
    } else if eq("industrial") {
        if capacity_kwh.is_some_and(|k| k.is_finite() && k > 2.0) {
            Art8Category::IndustrialEvSli
        } else {
            Art8Category::NotCovered
        }
    } else {
        // ev, sli / starting-lighting-ignition, and anything unrecognised.
        Art8Category::IndustrialEvSli
    }
}

// ── Input type ────────────────────────────────────────────────────────────────

/// Declared recycled content percentages for the four regulated metals.
///
/// `None` means the metal is absent or undeclared — it is skipped in target
/// checks. Only declared values can fail a target check.
#[derive(Debug, Clone, Copy)]
pub struct RecycledContentInput {
    pub cobalt_pct: Option<f64>,
    pub lithium_pct: Option<f64>,
    pub nickel_pct: Option<f64>,
    pub lead_pct: Option<f64>,
}

/// A recycled-content shortfall for a single material.
#[derive(Debug, Clone, Copy)]
pub struct RecycledContentShortfall {
    pub material: &'static str,
    pub declared_pct: f64,
    pub required_pct: f64,
}

// ── Phase-check functions ─────────────────────────────────────────────────────

/// Check declared recycled content against Art. 8(2) Phase 1 targets (from 2031).
///
/// Returns every material whose declared percentage falls below the Phase 1
/// minimum. An empty `Vec` means all declared metals pass. Undeclared metals
/// are not checked — battery-type scoping (Phase 1: EV / SLI / industrial
/// > 2 kWh; LMT only from Phase 2) is the caller's responsibility.
#[must_use]
pub fn art8_shortfalls_2031(input: &RecycledContentInput) -> Vec<RecycledContentShortfall> {
    check_targets(
        input,
        COBALT_RECYCLED_PCT_2031,
        LEAD_RECYCLED_PCT_2031,
        LITHIUM_RECYCLED_PCT_2031,
        NICKEL_RECYCLED_PCT_2031,
    )
}

/// Check declared recycled content against Art. 8(3) Phase 2 targets (from 2036).
#[must_use]
pub fn art8_shortfalls_2036(input: &RecycledContentInput) -> Vec<RecycledContentShortfall> {
    check_targets(
        input,
        COBALT_RECYCLED_PCT_2036,
        LEAD_RECYCLED_PCT_2036,
        LITHIUM_RECYCLED_PCT_2036,
        NICKEL_RECYCLED_PCT_2036,
    )
}

fn check_targets(
    input: &RecycledContentInput,
    cobalt_req: f64,
    lead_req: f64,
    lithium_req: f64,
    nickel_req: f64,
) -> Vec<RecycledContentShortfall> {
    let mut out = Vec::new();
    if let Some(pct) = input.cobalt_pct {
        // Non-finite (NaN/Inf) cannot demonstrate compliance — treat as shortfall.
        if !pct.is_finite() || pct < cobalt_req {
            out.push(RecycledContentShortfall {
                material: "cobalt",
                declared_pct: pct,
                required_pct: cobalt_req,
            });
        }
    }
    if let Some(pct) = input.lead_pct
        && (!pct.is_finite() || pct < lead_req)
    {
        out.push(RecycledContentShortfall {
            material: "lead",
            declared_pct: pct,
            required_pct: lead_req,
        });
    }
    if let Some(pct) = input.lithium_pct
        && (!pct.is_finite() || pct < lithium_req)
    {
        out.push(RecycledContentShortfall {
            material: "lithium",
            declared_pct: pct,
            required_pct: lithium_req,
        });
    }
    if let Some(pct) = input.nickel_pct
        && (!pct.is_finite() || pct < nickel_req)
    {
        out.push(RecycledContentShortfall {
            material: "nickel",
            declared_pct: pct,
            required_pct: nickel_req,
        });
    }
    out
}

// ── Chemistry → regulated-metal applicability ──────────────────────────────────

/// The Art. 8 regulated metals (cobalt, lithium, nickel, lead) that are
/// *meaningfully present* for a given battery chemistry.
///
/// Used to scope recycled-content checks so a chemistry that does not contain a
/// metal is never flagged for that metal's "shortfall" — e.g. an LFP cell
/// (LiFePO₄, no cobalt or nickel) must not produce a cobalt shortfall just
/// because the field defaulted to `0.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegulatedMetals {
    pub cobalt: bool,
    pub lithium: bool,
    pub nickel: bool,
    pub lead: bool,
}

/// Map a battery chemistry code (e.g. `"LFP"`, `"NMC"`, `"lead-acid"`) to the
/// Art. 8 regulated metals it contains.
///
/// Matching is case-insensitive. Unknown chemistries return **all `true`**
/// (conservative: every declared value is checked, since we cannot rule a metal
/// out). The caller still skips any metal whose declared percentage is absent.
#[must_use]
pub fn chemistry_regulated_metals(chemistry: &str) -> RegulatedMetals {
    let c = chemistry.trim();
    let eq = |s: &str| c.eq_ignore_ascii_case(s);
    if eq("LFP") {
        RegulatedMetals {
            cobalt: false,
            lithium: true,
            nickel: false,
            lead: false,
        }
    } else if eq("NMC") || eq("NCA") {
        RegulatedMetals {
            cobalt: true,
            lithium: true,
            nickel: true,
            lead: false,
        }
    } else if eq("LCO") {
        RegulatedMetals {
            cobalt: true,
            lithium: true,
            nickel: false,
            lead: false,
        }
    } else if eq("NiMH") || eq("NiCd") {
        RegulatedMetals {
            cobalt: false,
            lithium: false,
            nickel: true,
            lead: false,
        }
    } else if eq("lead-acid") {
        RegulatedMetals {
            cobalt: false,
            lithium: false,
            nickel: false,
            lead: true,
        }
    } else if eq("solid-state") {
        RegulatedMetals {
            cobalt: false,
            lithium: true,
            nickel: false,
            lead: false,
        }
    } else {
        // Unknown chemistry — cannot exclude any metal; check whatever is declared.
        RegulatedMetals {
            cobalt: true,
            lithium: true,
            nickel: true,
            lead: true,
        }
    }
}

/// Metals whose recycled content is declared **> 0** but which the chemistry
/// does **not** contain — a data-integrity contradiction (e.g. cobalt recycled
/// content on an LFP cell, which has no cobalt).
///
/// A declared `0.0` is *not* a conflict (it states "no recycled content", which
/// is trivially true for an absent metal). Unknown chemistries contain every
/// metal per [`chemistry_regulated_metals`], so they never conflict.
#[must_use]
pub fn recycled_content_chemistry_conflicts(
    chemistry: &str,
    cobalt_pct: Option<f64>,
    lithium_pct: Option<f64>,
    nickel_pct: Option<f64>,
    lead_pct: Option<f64>,
) -> Vec<&'static str> {
    let reg = chemistry_regulated_metals(chemistry);
    let positive = |v: Option<f64>| matches!(v, Some(x) if x.is_finite() && x > 0.0);
    let mut out = Vec::new();
    if positive(cobalt_pct) && !reg.cobalt {
        out.push("cobalt");
    }
    if positive(lithium_pct) && !reg.lithium {
        out.push("lithium");
    }
    if positive(nickel_pct) && !reg.nickel {
        out.push("nickel");
    }
    if positive(lead_pct) && !reg.lead {
        out.push("lead");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_metals(co: f64, pb: f64, li: f64, ni: f64) -> RecycledContentInput {
        RecycledContentInput {
            cobalt_pct: Some(co),
            lead_pct: Some(pb),
            lithium_pct: Some(li),
            nickel_pct: Some(ni),
        }
    }

    // ── Art. 8 phase selection — golden vectors ──────────────────────────────

    #[test]
    fn battery_placed_before_phase1_stays_unbound_however_late_it_is_assessed() {
        // A battery lawfully placed on the EU market on 1 Jun 2030, assessed in
        // 2033. Art. 8(2) attaches at placing on the market, so the 2031 minimums
        // never reach it. Deriving the phase from the assessment date instead
        // would report this battery as non-compliant from 18 Aug 2031 onwards.
        let placed = CalendarDate::new(2030, 6, 1);
        assert_eq!(
            art8_phase_for(Art8Category::IndustrialEvSli, placed, Art8SecondLife::None),
            Art8Phase::NotYetBinding
        );
    }

    #[test]
    fn phase1_boundary_is_inclusive_of_18_august_2031() {
        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                CalendarDate::new(2031, 8, 17),
                Art8SecondLife::None
            ),
            Art8Phase::NotYetBinding
        );
        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                CalendarDate::new(2031, 8, 18),
                Art8SecondLife::None
            ),
            Art8Phase::Phase1
        );
    }

    #[test]
    fn phase2_boundary_is_inclusive_of_18_august_2036() {
        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                CalendarDate::new(2036, 8, 17),
                Art8SecondLife::None
            ),
            Art8Phase::Phase1
        );
        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                CalendarDate::new(2036, 8, 18),
                Art8SecondLife::None
            ),
            Art8Phase::Phase2
        );
    }

    #[test]
    fn lmt_is_outside_phase1_entirely() {
        // Art. 8(2) does not name LMT batteries; Art. 8(3) does. An LMT battery
        // placed on the market the very day Phase 1 begins is still unbound.
        assert_eq!(
            art8_phase_for(
                Art8Category::Lmt,
                CalendarDate::new(2031, 8, 18),
                Art8SecondLife::None
            ),
            Art8Phase::NotYetBinding
        );
        assert_eq!(
            art8_phase_for(
                Art8Category::Lmt,
                CalendarDate::new(2036, 8, 18),
                Art8SecondLife::None
            ),
            Art8Phase::Phase2
        );
    }

    #[test]
    fn out_of_scope_categories_are_never_bound() {
        // Portable batteries, at any date, including well past Phase 2.
        assert_eq!(
            art8_phase_for(
                Art8Category::NotCovered,
                CalendarDate::new(2040, 1, 1),
                Art8SecondLife::None
            ),
            Art8Phase::NotCovered
        );
    }

    #[test]
    fn not_covered_and_not_yet_binding_are_distinguishable() {
        // The whole point of a four-outcome enum: an operator told "not covered"
        // and one told "not yet" have different obligations, and neither has a
        // shortfall.
        let portable = art8_phase_for(
            Art8Category::NotCovered,
            CalendarDate::new(2033, 1, 1),
            Art8SecondLife::None,
        );
        let early_ev = art8_phase_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2030, 1, 1),
            Art8SecondLife::None,
        );
        assert_ne!(portable, early_ev);
    }

    #[test]
    fn selected_phase_agrees_with_the_target_constants() {
        // 16 % cobalt clears Phase 1 and fails Phase 2. The phase selector and
        // the constants it selects between must not disagree about which set
        // is in force on a given placing-on-market date.
        let input = all_metals(16.0, 85.0, 6.0, 6.0);

        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                CalendarDate::new(2032, 1, 1),
                Art8SecondLife::None
            ),
            Art8Phase::Phase1
        );
        assert!(art8_shortfalls_2031(&input).is_empty());

        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                CalendarDate::new(2037, 1, 1),
                Art8SecondLife::None
            ),
            Art8Phase::Phase2
        );
        assert!(
            art8_shortfalls_2036(&input)
                .iter()
                .any(|s| s.material == "cobalt")
        );
    }

    // ── Category mapping ─────────────────────────────────────────────────────

    #[test]
    fn portable_is_not_covered_and_lmt_is_its_own_category() {
        assert_eq!(
            art8_category_for("portable", None),
            Art8Category::NotCovered
        );
        assert_eq!(
            art8_category_for("PORTABLE", None),
            Art8Category::NotCovered
        );
        assert_eq!(art8_category_for("lmt", None), Art8Category::Lmt);
        assert_eq!(art8_category_for(" LMT ", None), Art8Category::Lmt);
    }

    #[test]
    fn industrial_is_covered_only_above_two_kwh() {
        assert_eq!(
            art8_category_for("industrial", Some(2.5)),
            Art8Category::IndustrialEvSli
        );
        // Art. 8 says "greater than 2 kWh" — exactly 2 kWh is out.
        assert_eq!(
            art8_category_for("industrial", Some(2.0)),
            Art8Category::NotCovered
        );
        assert_eq!(
            art8_category_for("industrial", None),
            Art8Category::NotCovered
        );
        // A non-finite capacity cannot demonstrate the threshold is met.
        assert_eq!(
            art8_category_for("industrial", Some(f64::NAN)),
            Art8Category::NotCovered
        );
    }

    #[test]
    fn ev_sli_and_unknown_types_are_assessed() {
        for t in ["ev", "sli", "starting-lighting-ignition", "", "mystery"] {
            assert_eq!(
                art8_category_for(t, None),
                Art8Category::IndustrialEvSli,
                "type {t:?} should be assessed"
            );
        }
    }

    // ── Art. 8(1) declaration duty ───────────────────────────────────────────

    #[test]
    fn declaration_is_certainly_not_due_before_its_floor() {
        // The one thing that *is* knowable while the delegated act is
        // unadopted: the real date is never earlier than the floor, so anything
        // placed on the market before it definitely owes nothing yet.
        assert_eq!(
            art8_declaration_duty_for(
                Art8Category::IndustrialEvSli,
                CalendarDate::new(2028, 8, 17)
            ),
            Art8DeclarationDuty::NotYetDue {
                not_before: CalendarDate::new(2028, 8, 18)
            }
        );
    }

    #[test]
    fn declaration_is_undetermined_on_and_after_the_floor() {
        // Art. 8(1) applies from the floor "or 24 months after the date of entry
        // into force of the delegated act, whichever is the latest". The act is
        // unadopted, so on/after the floor the answer is genuinely unknown —
        // reporting it as "due" would assert a date the regulation does not set.
        let got =
            art8_declaration_duty_for(Art8Category::IndustrialEvSli, CalendarDate::new(2029, 1, 1));
        let Art8DeclarationDuty::Undetermined {
            not_before,
            empowerment,
        } = got
        else {
            panic!("expected Undetermined, got {got:?}");
        };
        assert_eq!(not_before, CalendarDate::new(2028, 8, 18));
        assert!(empowerment.contains("Art. 8(1)"));
    }

    #[test]
    fn lmt_declaration_floor_is_five_years_later() {
        // Art. 8(1) second subparagraph: LMT from 18 Aug 2033, not 2028.
        assert_eq!(
            art8_declaration_duty_for(Art8Category::Lmt, CalendarDate::new(2030, 1, 1)),
            Art8DeclarationDuty::NotYetDue {
                not_before: CalendarDate::new(2033, 8, 18)
            }
        );
        assert!(matches!(
            art8_declaration_duty_for(Art8Category::Lmt, CalendarDate::new(2034, 1, 1)),
            Art8DeclarationDuty::Undetermined { .. }
        ));
    }

    #[test]
    fn out_of_scope_categories_owe_no_declaration() {
        assert_eq!(
            art8_declaration_duty_for(Art8Category::NotCovered, CalendarDate::new(2040, 1, 1)),
            Art8DeclarationDuty::NotCovered
        );
    }

    #[test]
    fn the_declaration_and_minimum_ladders_are_independent() {
        // A battery placed on the market in 2029 is past the declaration floor
        // but years short of the Art. 8(2) minimums. Conflating the two would
        // report a shortfall against a duty that does not yet exist.
        let placed = CalendarDate::new(2029, 1, 1);
        assert!(matches!(
            art8_declaration_duty_for(Art8Category::IndustrialEvSli, placed),
            Art8DeclarationDuty::Undetermined { .. }
        ));
        assert_eq!(
            art8_phase_for(Art8Category::IndustrialEvSli, placed, Art8SecondLife::None),
            Art8Phase::NotYetBinding
        );
    }

    // ── Target checks ────────────────────────────────────────────────────────

    #[test]
    fn exactly_at_2031_targets_passes() {
        let input = all_metals(16.0, 85.0, 6.0, 6.0);
        assert!(art8_shortfalls_2031(&input).is_empty());
    }

    #[test]
    fn above_2031_targets_passes() {
        let input = all_metals(20.0, 90.0, 10.0, 10.0);
        assert!(art8_shortfalls_2031(&input).is_empty());
    }

    #[test]
    fn below_2031_cobalt_flagged() {
        let input = all_metals(15.0, 85.0, 6.0, 6.0); // cobalt 15 < 16
        let shortfalls = art8_shortfalls_2031(&input);
        assert_eq!(shortfalls.len(), 1);
        assert_eq!(shortfalls[0].material, "cobalt");
        assert_eq!(shortfalls[0].required_pct, 16.0);
    }

    #[test]
    fn multiple_shortfalls_all_returned() {
        let input = all_metals(10.0, 80.0, 3.0, 4.0); // all below
        assert_eq!(art8_shortfalls_2031(&input).len(), 4);
    }

    #[test]
    fn undeclared_metals_not_flagged() {
        let input = RecycledContentInput {
            cobalt_pct: Some(20.0),
            lead_pct: None,
            lithium_pct: None,
            nickel_pct: None,
        };
        assert!(art8_shortfalls_2031(&input).is_empty());
    }

    #[test]
    fn phase2_stricter_than_phase1() {
        // 16% cobalt passes 2031 but fails 2036 (target 26%)
        let input = all_metals(16.0, 85.0, 6.0, 6.0);
        assert!(art8_shortfalls_2031(&input).is_empty());
        let shortfalls = art8_shortfalls_2036(&input);
        assert!(shortfalls.iter().any(|s| s.material == "cobalt"));
    }

    #[test]
    fn nan_cobalt_treated_as_shortfall() {
        let input = RecycledContentInput {
            cobalt_pct: Some(f64::NAN),
            lead_pct: None,
            lithium_pct: None,
            nickel_pct: None,
        };
        let shortfalls = art8_shortfalls_2031(&input);
        assert_eq!(shortfalls.len(), 1);
        assert_eq!(shortfalls[0].material, "cobalt");
    }

    #[test]
    fn infinity_cobalt_treated_as_shortfall() {
        let input = RecycledContentInput {
            cobalt_pct: Some(f64::INFINITY),
            lead_pct: None,
            lithium_pct: None,
            nickel_pct: None,
        };
        let shortfalls = art8_shortfalls_2031(&input);
        assert_eq!(shortfalls.len(), 1);
        assert_eq!(shortfalls[0].material, "cobalt");
    }

    #[test]
    fn lfp_regulates_lithium_only() {
        let m = chemistry_regulated_metals("LFP");
        assert!(m.lithium);
        assert!(!m.cobalt && !m.nickel && !m.lead);
        // case-insensitive
        assert_eq!(chemistry_regulated_metals("lfp"), m);
    }

    #[test]
    fn nmc_and_nca_regulate_cobalt_lithium_nickel() {
        for chem in ["NMC", "NCA"] {
            let m = chemistry_regulated_metals(chem);
            assert!(m.cobalt && m.lithium && m.nickel);
            assert!(!m.lead);
        }
    }

    #[test]
    fn lead_acid_regulates_lead_only() {
        let m = chemistry_regulated_metals("lead-acid");
        assert!(m.lead);
        assert!(!m.cobalt && !m.lithium && !m.nickel);
    }

    #[test]
    fn unknown_chemistry_checks_all_metals() {
        let m = chemistry_regulated_metals("mystery-cell");
        assert!(m.cobalt && m.lithium && m.nickel && m.lead);
    }

    #[test]
    fn positive_cobalt_on_lfp_is_a_conflict() {
        let c = recycled_content_chemistry_conflicts("LFP", Some(5.0), Some(12.0), None, None);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0], "cobalt");
    }

    #[test]
    fn zero_cobalt_on_lfp_is_not_a_conflict() {
        // 0.0 declares "no recycled cobalt" — trivially true for an absent metal.
        let c = recycled_content_chemistry_conflicts("LFP", Some(0.0), Some(12.0), Some(0.0), None);
        assert!(c.is_empty(), "got: {c:?}");
    }

    #[test]
    fn nmc_cobalt_and_nickel_declared_no_conflict() {
        let c = recycled_content_chemistry_conflicts("NMC", Some(16.0), Some(6.0), Some(8.0), None);
        assert!(c.is_empty(), "got: {c:?}");
    }

    #[test]
    fn lead_declared_on_lfp_is_a_conflict() {
        let c = recycled_content_chemistry_conflicts("LFP", None, Some(12.0), None, Some(80.0));
        assert_eq!(c.len(), 1);
        assert_eq!(c[0], "lead");
    }

    #[test]
    fn unknown_chemistry_never_conflicts() {
        let c = recycled_content_chemistry_conflicts(
            "mystery",
            Some(5.0),
            Some(5.0),
            Some(5.0),
            Some(5.0),
        );
        assert!(c.is_empty());
    }

    // ── Art. 8(4) — the second-life carve-out ────────────────────────────────
    //
    // Added 2026-08-27. Before it, a remanufactured battery was reported as
    // bound by the minimum shares that Art. 8(4) removes.

    /// Art. 8(4) disapplies paragraphs 1-3 outright, so it answers before
    /// category or date. A battery deep inside Phase 2 is still exempt.
    #[test]
    fn a_second_life_battery_is_exempt_even_inside_phase_2() {
        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                CalendarDate::new(2040, 1, 1),
                Art8SecondLife::PlacedBeforeOperations,
            ),
            Art8Phase::ExemptSecondLife
        );
    }

    /// **Art. 8(4) is broader than Art. 10(4).** Its condition is prior
    /// placement relative to the *operation*, not to the obligation dates, so a
    /// battery placed in 2033 and remanufactured later is exempt from Art. 8 —
    /// where the same battery is not exempt from Art. 10. Collapsing the two
    /// carve-outs into one test would hide exactly this.
    #[test]
    fn art8_exemption_does_not_require_placement_before_the_phase_dates() {
        let placed_after_phase1_began = CalendarDate::new(2033, 5, 1);
        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                placed_after_phase1_began,
                Art8SecondLife::None,
            ),
            Art8Phase::Phase1,
            "an original battery placed in 2033 is bound by Art. 8(2)"
        );
        assert_eq!(
            art8_phase_for(
                Art8Category::IndustrialEvSli,
                placed_after_phase1_began,
                Art8SecondLife::PlacedBeforeOperations,
            ),
            Art8Phase::ExemptSecondLife,
            "the same battery, remanufactured, is exempt under Art. 8(4)"
        );
    }

    /// Exempt and NotCovered are different sentences and must stay distinct.
    #[test]
    fn exempt_is_not_the_same_answer_as_not_covered() {
        assert_ne!(Art8Phase::ExemptSecondLife, Art8Phase::NotCovered);
        assert_eq!(
            art8_phase_for(
                Art8Category::NotCovered,
                CalendarDate::new(2040, 1, 1),
                Art8SecondLife::PlacedBeforeOperations,
            ),
            Art8Phase::ExemptSecondLife,
            "Art. 8(4) is checked first, so it answers even for an out-of-scope category"
        );
    }
}

// ── Art. 8(1) — the declaration duty ─────────────────────────────────────────

/// Floor date for the Art. 8(1) documentation duty for industrial batteries
/// > 2 kWh, EV and SLI — 18 August 2028.
pub const ART8_DECLARATION_FLOOR_2028: CalendarDate = CalendarDate::new(2028, 8, 18);

/// Floor date for the Art. 8(1) documentation duty for LMT batteries —
/// 18 August 2033.
pub const ART8_DECLARATION_FLOOR_2033: CalendarDate = CalendarDate::new(2033, 8, 18);

/// The empowerment whose entry into force can push the Art. 8(1) duty later.
pub const ART8_DECLARATION_EMPOWERMENT: &str = "EU 2023/1542 Art. 8(1), third subparagraph — recycled content calculation,      verification and documentation format";

/// Whether the Art. 8(1) documentation duty has begun for a battery.
///
/// Unlike the Art. 8(2)/(3) minimums, whose dates are stated outright, Art. 8(1)
/// applies from *"18 August 2028 or 24 months after the date of entry into force
/// of the delegated act …, whichever is the latest"*. That act has not been
/// adopted, so the real start date is unknown — but it is never **earlier** than
/// the floor, which is enough to answer the question for anything placed on the
/// market before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art8DeclarationDuty {
    /// Art. 8(1) does not reach this category.
    NotCovered,
    /// Certainly not yet owed: placed on the market before the floor date, and
    /// the real date can only be that floor or later.
    NotYetDue { not_before: CalendarDate },
    /// Cannot be determined. Placed on the market on or after the floor, but
    /// whether the duty had begun depends on when the delegated act entered
    /// into force — and it has not been adopted.
    Undetermined {
        not_before: CalendarDate,
        empowerment: &'static str,
    },
}

/// Whether a battery owes the Art. 8(1) recycled-content declaration.
///
/// Keyed on the date the battery was **placed on the EU market**, like every
/// other Art. 8 duty — not on the date of assessment.
#[must_use]
pub fn art8_declaration_duty_for(
    category: Art8Category,
    placed_on_market: CalendarDate,
) -> Art8DeclarationDuty {
    let floor = match category {
        Art8Category::NotCovered => return Art8DeclarationDuty::NotCovered,
        Art8Category::IndustrialEvSli => ART8_DECLARATION_FLOOR_2028,
        Art8Category::Lmt => ART8_DECLARATION_FLOOR_2033,
    };
    if placed_on_market < floor {
        Art8DeclarationDuty::NotYetDue { not_before: floor }
    } else {
        Art8DeclarationDuty::Undetermined {
            not_before: floor,
            empowerment: ART8_DECLARATION_EMPOWERMENT,
        }
    }
}
