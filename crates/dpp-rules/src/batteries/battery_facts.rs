//! [`BatteryFacts`] — what the content table needs to know about one battery
//! beyond its category.

use super::due_diligence::{DueDiligenceSecondLife, OperatorScope};
use crate::common::date::CalendarDate;

/// The facts about one battery that a scheduled row of the content table reads.
///
/// `Default` is the state of a record that has said nothing: no date, no
/// second-life history, no declaration. Every row that depends on one of those
/// answers
/// [`Requirement::Undetermined`](super::passport_content::Requirement::Undetermined)
/// for it rather than guessing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BatteryFacts {
    /// The placing-on-market (or putting-into-service) date. Never the date of
    /// assessment: see [`super::due_diligence`].
    pub placed_on_market: Option<CalendarDate>,
    /// Whether Art. 47's second paragraph reaches the battery.
    pub second_life: DueDiligenceSecondLife,
    /// The operator's declaration against Art. 47's first paragraph.
    pub operator_scope: Option<OperatorScope>,
}
