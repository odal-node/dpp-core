//! Who owes a battery passport at all — Regulation (EU) 2023/1542 **Art. 77(1)**.
//!
//! Read from the consolidated text `02023R1542 -- EN -- 31.07.2025 -- 002.004`,
//! **p. 82**, verbatim:
//!
//! > "From 18 February 2027 each LMT battery, each industrial battery with a
//! > capacity greater than 2 kWh and each electric vehicle battery placed on the
//! > market or put into service shall have an electronic record ('battery
//! > passport')."
//!
//! ## Three of five categories, and the one qualifier
//!
//! The Regulation defines five battery categories. Art. 77(1) reaches **three**:
//! LMT, electric-vehicle, and industrial **above 2 kWh**. Portable and SLI
//! batteries are named nowhere in it and owe no passport.
//!
//! 🚨 **It says "each industrial battery", not "each *rechargeable* industrial
//! battery".** Art. 7(2) and Art. 10(5) both add "rechargeable" when they mean
//! it; Art. 77(1) does not. So the passport scope is every industrial battery
//! over the threshold, and narrowing it to rechargeable ones would exempt
//! batteries the article covers.
//!
//! ## Why this is not [`passport_content`](super::passport_content)
//!
//! That module answers *what a battery of this category must carry*. It returns
//! [`Requirement::Unknown`](super::passport_content::Requirement::Unknown) for
//! portable and SLI, because the Commission's data-point guidance does not cover
//! them — which is the honest answer to the question it is asked.
//!
//! But a content gate reading `Unknown` demands nothing, so a portable battery
//! passes it while claiming to discharge an obligation that does not exist. The
//! two questions have to be asked separately: *is a passport owed?* comes first,
//! and only then *what must it contain?*

use crate::common::date::CalendarDate;

/// Capacity above which an industrial battery owes a passport, in kWh.
///
/// Art. 77(1), "a capacity **greater than** 2 kWh" — strictly greater, so a
/// battery at exactly 2 kWh owes nothing.
///
/// The same figure scopes Art. 8 recycled content and Art. 7(2) carbon-footprint
/// labelling, and it is deliberately restated here rather than shared: three
/// articles that happen to agree today are three articles that can be amended
/// separately, and a single constant would hide the day one of them moves.
pub const INDUSTRIAL_PASSPORT_THRESHOLD_KWH: f64 = 2.0;

/// First day the Art. 77(1) passport obligation applies — 18 February 2027.
pub const PASSPORT_REQUIRED_FROM: CalendarDate = CalendarDate::new(2027, 2, 18);

/// Whether a battery owes a passport under Art. 77(1), and if not, why not.
///
/// Five outcomes, deliberately not a `bool`. "This category never owes one",
/// "this one is under the threshold", "we cannot tell" and "not yet" are four
/// different sentences to put in front of an operator, and collapsing any of
/// them into `false` reports an exemption the Regulation does not grant — or
/// hides one it does. The same reasoning shapes
/// [`Art8Phase`](super::recycled_content::Art8Phase).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PassportScope {
    /// Art. 77(1) does not name this category — portable and SLI. No passport is
    /// owed, ever, and none of the Annex XIII content requirements apply.
    NotCovered,
    /// An industrial battery at or below 2 kWh. The category is named; this unit
    /// is under the threshold.
    ///
    /// Distinct from [`NotCovered`](Self::NotCovered): the category *is* in
    /// scope and a larger battery of the same kind would owe a passport, which
    /// is a different thing to tell an operator than "industrial batteries are
    /// outside Art. 77".
    BelowThreshold,
    /// An industrial battery whose capacity was not stated, so the threshold
    /// cannot be applied.
    ///
    /// **Not an exemption.** The obligation turns on a number this record does
    /// not carry, and answering `NotCovered` would exempt a battery on the
    /// strength of a missing field. A caller must treat this as "ask for the
    /// capacity", never as "no passport needed" — the same fail-closed direction
    /// that makes an unmarked retention figure `Assumed`.
    CapacityUnknown,
    /// In scope, but placed on the market before 18 February 2027.
    NotYetBinding,
    /// A passport is owed.
    Required,
}

impl PassportScope {
    /// Whether a passport is owed **now**, on the strength of this answer alone.
    ///
    /// `true` only for [`Required`](Self::Required). Every other variant is a
    /// reason no passport is owed *or* a reason the question is unanswered, and
    /// a caller that needs to tell those apart must match on the variant —
    /// which is why this deliberately does not return `Option<bool>`.
    #[must_use]
    pub fn is_required(self) -> bool {
        matches!(self, Self::Required)
    }

    /// Whether the answer turns on information the record did not supply.
    ///
    /// Exists so a caller can route the one recoverable case — ask for the
    /// capacity — without matching every variant.
    #[must_use]
    pub fn is_undetermined(self) -> bool {
        matches!(self, Self::CapacityUnknown)
    }
}

/// Whether `battery_type` owes a battery passport under Art. 77(1).
///
/// `battery_type` is the **wire** name — `"ev"`, `"lmt"`, `"industrial"`,
/// `"portable"`, `"starting-lighting-ignition"` — because this crate is `no_std`
/// and zero-dependency and is read by the Wasm product group plugins, which see
/// JSON and never the Rust enum. Matching is case-insensitive and trims
/// surrounding whitespace, as in [`super::passport_content`].
///
/// `capacity_kwh` is consulted **only** for industrial batteries; it is ignored
/// for every other category, because Art. 77(1) attaches the threshold to that
/// word alone. `None` for an industrial battery yields
/// [`PassportScope::CapacityUnknown`].
///
/// An unrecognised `battery_type` answers [`PassportScope::NotCovered`] — Art.
/// 77(1) is a closed list of three, so anything that is not one of them is
/// outside it. That is a statement about the article, not a guess about the
/// input.
#[must_use]
pub fn passport_scope(
    battery_type: &str,
    capacity_kwh: Option<f64>,
    placed_on_market: CalendarDate,
) -> PassportScope {
    let t = battery_type.trim();
    let eq = |s: &str| t.eq_ignore_ascii_case(s);

    let in_scope = if eq("ev") || eq("lmt") {
        true
    } else if eq("industrial") {
        match capacity_kwh {
            // NaN compares false against every bound, so it falls through to
            // `CapacityUnknown` rather than silently reading as "not above the
            // threshold" — an unusable number is not a statement of capacity.
            Some(kwh) if kwh > INDUSTRIAL_PASSPORT_THRESHOLD_KWH => true,
            Some(kwh) if kwh <= INDUSTRIAL_PASSPORT_THRESHOLD_KWH => {
                return PassportScope::BelowThreshold;
            }
            _ => return PassportScope::CapacityUnknown,
        }
    } else {
        // portable, sli / starting-lighting-ignition, and anything unrecognised.
        false
    };

    if !in_scope {
        return PassportScope::NotCovered;
    }
    if placed_on_market < PASSPORT_REQUIRED_FROM {
        return PassportScope::NotYetBinding;
    }
    PassportScope::Required
}
