//! Recycled-content shares required of battery materials, and when.

use alloc::vec::Vec;

use crate::batteries::schedule::{ChangeKind, ScheduledChange};
use crate::common::date::CalendarDate;

// BOUNDARY-EXCEPTION(taxonomy): "Phase 1" and "Phase 2" are the two stages of
// Regulation (EU) 2023/1542 Art. 8(2) and 8(3), with statutory dates — the
// Regulation's own staging, not a roadmap.
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

/// The changes [`ART8_PHASE1_FROM`] and [`ART8_PHASE2_FROM`] schedule.
///
/// The Art. 8(1) declaration floors ([`ART8_DECLARATION_FLOOR_2028`] and
/// [`ART8_DECLARATION_FLOOR_2033`]) are deliberately absent: each is the earliest
/// day that duty can begin, and the day it does begin turns on a delegated act
/// not yet adopted. A calendar that listed one would announce a start nobody
/// knows.
pub const SCHEDULE: &[ScheduledChange] = &[
    ScheduledChange {
        id: "battery.recycled-content.minimum-shares-2031",
        date: ART8_PHASE1_FROM,
        kind: ChangeKind::MinimumShare,
        citation: "Regulation (EU) 2023/1542 Art. 8(2)",
        summary: "Minimum recycled-content shares of cobalt, lead, lithium and nickel bind \
                  industrial batteries over 2 kWh (except those with exclusively external \
                  storage), electric-vehicle batteries and SLI batteries placed on the \
                  market from this date.",
        product_group: "battery",
        categories: &["industrial", "ev", "starting-lighting-ignition"],
    },
    ScheduledChange {
        id: "battery.recycled-content.minimum-shares-2036",
        date: ART8_PHASE2_FROM,
        kind: ChangeKind::MinimumShare,
        citation: "Regulation (EU) 2023/1542 Art. 8(3)",
        summary: "Higher minimum recycled-content shares bind the same categories, and LMT \
                  batteries for the first time, for batteries placed on the market from this \
                  date.",
        product_group: "battery",
        categories: &["industrial", "ev", "starting-lighting-ignition", "lmt"],
    },
];

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
/// Five outcomes, deliberately not collapsed into `Option`: "Art. 8 does not
/// reach this battery", "Art. 8 does not reach this battery *yet*" and "Art. 8
/// reaches this battery and Art. 8(4) excuses it" are three different answers,
/// and reporting any of them as a shortfall misstates an operator's legal
/// position.
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

// ── Art. 8(1) — the declaration duty ─────────────────────────────────────────

/// Floor date for the Art. 8(1) documentation duty for industrial batteries
/// > 2 kWh, EV and SLI — 18 August 2028.
pub const ART8_DECLARATION_FLOOR_2028: CalendarDate = CalendarDate::new(2028, 8, 18);

/// Floor date for the Art. 8(1) documentation duty for LMT batteries —
/// 18 August 2033.
pub const ART8_DECLARATION_FLOOR_2033: CalendarDate = CalendarDate::new(2033, 8, 18);

/// The empowerment whose entry into force can push the Art. 8(1) duty later.
pub const ART8_DECLARATION_EMPOWERMENT: &str = "EU 2023/1542 Art. 8(1), third subparagraph — \
     recycled content calculation, verification and documentation format";

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
