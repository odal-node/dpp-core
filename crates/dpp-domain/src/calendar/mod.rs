//! The dated-obligation calendar: which obligations take effect between two
//! dates.
//!
//! The dates that decide when an obligation binds live in two places, and this
//! module is the one place that can be asked about both:
//!
//! - **Rule modules** in `dpp_rules` hold `CalendarDate` constants and select
//!   their answer by them. Each lists the changes its constants schedule as a
//!   `SCHEDULE` slice built from those same constants, so the date a rule
//!   applies and the date this calendar reports are one value.
//! - **The instrument catalog** records when each act's passport obligation
//!   begins, as an [`ObligationDate`](crate::instrument::ObligationDate) with its
//!   [`DateBasis`](crate::instrument::DateBasis).
//!
//! # No clock
//!
//! Nothing here reads the current date. A caller passes both ends of the
//! window, so the same question always gets the same answer. The calendar says
//! what takes effect on a day; it never decides which law governs a passport,
//! which is the placing-on-market date's job.
//!
//! # What is not here
//!
//! A change with no date. An obligation whose start waits on an act not yet
//! adopted has nothing to announce, so it is not an event: an ESPR product group
//! whose delegated act does not exist, the battery data points awaiting an
//! implementing act, and the Art. 8(1) declaration duty, whose stated date is
//! only the earliest it can begin.

mod obligation_calendar;
mod obligation_event;
mod obligation_kind;
#[cfg(test)]
mod tests;

pub use obligation_calendar::ObligationCalendar;
pub use obligation_event::ObligationEvent;
pub use obligation_kind::ObligationKind;
