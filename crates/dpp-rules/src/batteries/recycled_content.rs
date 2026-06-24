//! Battery recycled content validation — EU Regulation 2023/1542, Art. 8 + Annex X.
//!
//! Art. 8 and Annex X set minimum recycled content targets for four metals.
//! Phase 1 (from 18 Aug 2031) covers **EV batteries, SLI batteries, and
//! industrial batteries with a capacity > 2 kWh** (excluding those with
//! exclusively external storage). **LMT batteries** join only in Phase 2 (from
//! **18 Aug 2036**), at the higher targets. Portable batteries are out of scope.
//!
//! The targets are **finalized law** — they are in the regulation text itself,
//! not in a pending delegated act. However, neither phase is yet in force.
//! The battery plugin therefore returns `NOT_ASSESSED` today; these constants
//! are the single source of truth that the plugin will check against once
//! enforcement begins.
//!
//! ## Phase 1 — EV + SLI + industrial > 2 kWh, from **18 Aug 2031** (Art. 8)
//! | Material | Minimum % |
//! |----------|-----------|
//! | Cobalt   |      16 % |
//! | Lead     |      85 % |
//! | Lithium  |       6 % |
//! | Nickel   |       6 % |
//!
//! ## Phase 2 — Phase 1 categories + **LMT**, from **18 Aug 2036** (Art. 8)
//! | Material | Minimum % |
//! |----------|-----------|
//! | Cobalt   |      26 % |
//! | Lead     |      85 % |
//! | Lithium  |      12 % |
//! | Nickel   |      15 % |

use alloc::vec::Vec;

// ✅ COMPLIANCE-PIN: EU 2023/1542, Art. 8 + Annex X (OJ L 2023/1542, 28 Jul 2023)
// Percentages: verified correct per Annex X.
// Phase-1 date: 18 Aug 2031. Phase-2 date: 18 Aug 2036.
// Category scope (corrected 2026-06-22, audit H-2): Phase 1 = EV + SLI + industrial
// > 2 kWh (excl. exclusively-external-storage); LMT batteries join only in Phase 2
// (LMT minimum content from 18 Aug 2036). SLI is **in** Phase-1 scope — a prior note
// here wrongly excluded it. Reconciled against multiple authoritative secondary
// sources (White & Case, EUR-Lex summary, GLEIF-independent battery guidance); the
// 🟠 residual is verbatim OJ Art. 8(2)/(3) confirmation, blocked here by EUR-Lex
// JavaScript rendering. Numeric percentages/dates are not in dispute.

// ── Phase 1 constants — EV + industrial ≥ 2 kWh from 18 Aug 2031 ─────────────

/// Minimum cobalt recycled content — Art. 8 + Annex X Phase 1, from 18 Aug 2031.
pub const COBALT_RECYCLED_PCT_2031: f64 = 16.0;
/// Minimum lead recycled content — Art. 8 + Annex X Phase 1, from 18 Aug 2031.
pub const LEAD_RECYCLED_PCT_2031: f64 = 85.0;
/// Minimum lithium recycled content — Art. 8 + Annex X Phase 1, from 18 Aug 2031.
pub const LITHIUM_RECYCLED_PCT_2031: f64 = 6.0;
/// Minimum nickel recycled content — Art. 8 + Annex X Phase 1, from 18 Aug 2031.
pub const NICKEL_RECYCLED_PCT_2031: f64 = 6.0;

// ── Phase 2 constants — EV + industrial ≥ 2 kWh + LMT from 18 Aug 2036 ───────

/// Minimum cobalt recycled content — Art. 8 + Annex X Phase 2, from 18 Aug 2036.
pub const COBALT_RECYCLED_PCT_2036: f64 = 26.0;
/// Minimum lead recycled content — Art. 8 + Annex X Phase 2, from 18 Aug 2036.
pub const LEAD_RECYCLED_PCT_2036: f64 = 85.0;
/// Minimum lithium recycled content — Art. 8 + Annex X Phase 2, from 18 Aug 2036.
pub const LITHIUM_RECYCLED_PCT_2036: f64 = 12.0;
/// Minimum nickel recycled content — Art. 8 + Annex X Phase 2, from 18 Aug 2036.
pub const NICKEL_RECYCLED_PCT_2036: f64 = 15.0;

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

/// Check declared recycled content against Annex X Phase 1 targets (from 2031).
///
/// Returns every material whose declared percentage falls below the Phase 1
/// minimum. An empty `Vec` means all declared metals pass. Undeclared metals
/// are not checked — battery-type scoping (Phase 1: EV / SLI / industrial
/// > 2 kWh; LMT only from Phase 2) is the caller's responsibility.
#[must_use]
pub fn annex_x_shortfalls_2031(input: &RecycledContentInput) -> Vec<RecycledContentShortfall> {
    check_targets(
        input,
        COBALT_RECYCLED_PCT_2031,
        LEAD_RECYCLED_PCT_2031,
        LITHIUM_RECYCLED_PCT_2031,
        NICKEL_RECYCLED_PCT_2031,
    )
}

/// Check declared recycled content against Annex X Phase 2 targets (from 2036).
#[must_use]
pub fn annex_x_shortfalls_2036(input: &RecycledContentInput) -> Vec<RecycledContentShortfall> {
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

    #[test]
    fn exactly_at_2031_targets_passes() {
        let input = all_metals(16.0, 85.0, 6.0, 6.0);
        assert!(annex_x_shortfalls_2031(&input).is_empty());
    }

    #[test]
    fn above_2031_targets_passes() {
        let input = all_metals(20.0, 90.0, 10.0, 10.0);
        assert!(annex_x_shortfalls_2031(&input).is_empty());
    }

    #[test]
    fn below_2031_cobalt_flagged() {
        let input = all_metals(15.0, 85.0, 6.0, 6.0); // cobalt 15 < 16
        let shortfalls = annex_x_shortfalls_2031(&input);
        assert_eq!(shortfalls.len(), 1);
        assert_eq!(shortfalls[0].material, "cobalt");
        assert_eq!(shortfalls[0].required_pct, 16.0);
    }

    #[test]
    fn multiple_shortfalls_all_returned() {
        let input = all_metals(10.0, 80.0, 3.0, 4.0); // all below
        assert_eq!(annex_x_shortfalls_2031(&input).len(), 4);
    }

    #[test]
    fn undeclared_metals_not_flagged() {
        let input = RecycledContentInput {
            cobalt_pct: Some(20.0),
            lead_pct: None,
            lithium_pct: None,
            nickel_pct: None,
        };
        assert!(annex_x_shortfalls_2031(&input).is_empty());
    }

    #[test]
    fn phase2_stricter_than_phase1() {
        // 16% cobalt passes 2031 but fails 2036 (target 26%)
        let input = all_metals(16.0, 85.0, 6.0, 6.0);
        assert!(annex_x_shortfalls_2031(&input).is_empty());
        let shortfalls = annex_x_shortfalls_2036(&input);
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
        let shortfalls = annex_x_shortfalls_2031(&input);
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
        let shortfalls = annex_x_shortfalls_2031(&input);
        assert_eq!(shortfalls.len(), 1);
        assert_eq!(shortfalls[0].material, "cobalt");
    }
}
