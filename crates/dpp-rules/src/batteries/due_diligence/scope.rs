//! [`DueDiligenceScope`] — whether a battery's passport owes the Art. 52(3)
//! report, and [`due_diligence_scope`], which answers it.
//!
//! ✅ COMPLIANCE-PIN: EU 2023/1542, Art. 48(1) as replaced by Regulation (EU)
//! 2025/1561 (consolidated text 31.07.2025, p. 45), read on 2026-10-21: the duty
//! applies from 18 August 2027.

use super::{DueDiligenceSecondLife, OperatorScope};
use crate::batteries::schedule::{ChangeKind, ScheduledChange};
use crate::common::date::CalendarDate;

/// First day Art. 48(1) applies — 18 August 2027, as replaced by Regulation
/// (EU) 2025/1561.
pub const DUE_DILIGENCE_REQUIRED_FROM: CalendarDate = CalendarDate::new(2027, 8, 18);

/// The change [`DUE_DILIGENCE_REQUIRED_FROM`] schedules.
pub const SCHEDULE: &[ScheduledChange] = &[ScheduledChange {
    id: "battery.due-diligence-report",
    date: DUE_DILIGENCE_REQUIRED_FROM,
    kind: ChangeKind::ContentRequirement,
    citation: "Regulation (EU) 2023/1542 Art. 48(1), as replaced by Regulation (EU) \
               2025/1561, and Annex XIII point 1(d)",
    summary: "The passport of a battery placed on the market from this date carries the \
              address of its operator's due diligence report, unless Art. 47 takes the \
              operator or the battery out of the due diligence chapter.",
    product_group: "battery",
    categories: &["ev", "lmt", "industrial"],
}];

/// Whether a battery's passport owes the Art. 52(3) report, and if not, why not.
///
/// Six outcomes, not a `bool`, for the reason
/// [`PassportScope`](crate::batteries::passport_scope::PassportScope) gives:
/// "exempt", "not yet", and "we were not told" are different sentences to put in
/// front of an operator, and collapsing any of them reports an exemption the
/// Regulation does not grant, or hides one it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DueDiligenceScope {
    /// Art. 47, second paragraph: a second-life battery that was already on the
    /// market. Chapter VII does not apply to it at any date.
    ExemptSecondLife,
    /// The placing-on-market date was not given, so Art. 48(1) cannot be applied.
    /// **Not an exemption.**
    PlacingDateUnknown,
    /// Placed on the market before 18 August 2027. No report is owed for this
    /// battery, and the guidance says the field is not to be filled.
    NotYetBinding,
    /// Placed from 18 August 2027, but the operator has not declared its
    /// [`OperatorScope`]. **Not an exemption.**
    OperatorScopeUndeclared,
    /// Art. 47, first paragraph: the operator declares it is under the turnover
    /// threshold.
    ExemptBelowTurnoverThreshold,
    /// The report is owed, and the passport must carry its address.
    Required,
}

impl DueDiligenceScope {
    /// Whether the answer turns on a fact the caller did not supply.
    #[must_use]
    pub fn is_undetermined(self) -> bool {
        matches!(
            self,
            Self::PlacingDateUnknown | Self::OperatorScopeUndeclared
        )
    }
}

/// Whether a battery's passport owes the due diligence report.
///
/// The second-life limb answers first, because Art. 47's second paragraph does
/// not depend on any date: a second-life battery is out of the chapter whether
/// or not its placing date was given. The date answers next, and the operator's
/// declaration only matters once the date has been reached, so a battery placed
/// before 18 August 2027 is never asked for one.
///
/// The categories are not inputs. Art. 48(1) names "batteries" without
/// distinguishing them, and which categories carry a passport at all is
/// [`passport_scope`](crate::batteries::passport_scope::passport_scope)'s
/// question.
#[must_use]
pub fn due_diligence_scope(
    placed_on_market: Option<CalendarDate>,
    second_life: DueDiligenceSecondLife,
    operator: Option<OperatorScope>,
) -> DueDiligenceScope {
    if second_life == DueDiligenceSecondLife::PlacedBeforeOperations {
        return DueDiligenceScope::ExemptSecondLife;
    }
    let Some(placed) = placed_on_market else {
        return DueDiligenceScope::PlacingDateUnknown;
    };
    if placed < DUE_DILIGENCE_REQUIRED_FROM {
        return DueDiligenceScope::NotYetBinding;
    }
    match operator {
        None => DueDiligenceScope::OperatorScopeUndeclared,
        Some(OperatorScope::BelowTurnoverThreshold) => {
            DueDiligenceScope::ExemptBelowTurnoverThreshold
        }
        Some(OperatorScope::Subject) => DueDiligenceScope::Required,
    }
}
