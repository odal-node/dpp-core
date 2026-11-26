//! The clock an assessment reads its date from.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// The two dates a compliance computation needs. They are never the same field.
///
/// EU staged obligations attach at a regulated triggering event — for
/// Regulation (EU) 2023/1542 Articles 7, 8 and 10 that is placing on the market
/// or putting into service. Which law governs a given product is therefore fixed
/// at that moment, and does not change when the product is reassessed years
/// later. A battery lawfully placed on the market in 2030 does not acquire the
/// 18 August 2031 minimums by being audited in 2033.
///
/// There is deliberately **no `AssessmentClock::now()`**. A wall-clock
/// constructor is precisely the mistake this type exists to prevent: it would
/// let every determination silently re-derive the governing law from whichever
/// day the calculation happened to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentClock {
    /// Which law applies. Derived from the regulated triggering event — never
    /// from today's date.
    pub law_in_force_on: NaiveDate,
    /// When this computation ran. Receipt provenance only; this never selects a
    /// ruleset.
    pub computed_at: DateTime<Utc>,
}

impl AssessmentClock {
    /// Build a clock for a product whose governing law was fixed on
    /// `law_in_force_on`, computed now.
    ///
    /// The wall clock supplies `computed_at` and nothing else.
    /// `law_in_force_on` must come from the product's own record — the
    /// passport's placing-on-market date — and never from `Utc::now()`.
    #[must_use]
    pub fn placed_on(law_in_force_on: NaiveDate) -> Self {
        Self {
            law_in_force_on,
            computed_at: Utc::now(),
        }
    }

    /// Fully explicit constructor, pinning both dates.
    ///
    /// Re-verifying a stored receipt needs this: reproducing a historical
    /// calculation means reproducing the computation timestamp too, not just
    /// the governing law.
    #[must_use]
    pub const fn new(law_in_force_on: NaiveDate, computed_at: DateTime<Utc>) -> Self {
        Self {
            law_in_force_on,
            computed_at,
        }
    }
}
