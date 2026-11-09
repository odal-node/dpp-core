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
//! # Dates from a signed bundle
//!
//! A build knows the dates it was compiled with. With the `bundle` feature, a
//! verified signed ruleset bundle can carry more, so a node learns a new or
//! amended date without a crate release. The bundle fills what this build does
//! not hold, or holds only as an assumption, and never overrides a sourced
//! date: this build's rules select their answers by those, and a calendar that
//! announced a different date would contradict its own publish gate. Where a
//! bundle disagrees with one, the disagreement is kept as a
//! [`ScheduleConflict`].
//!
//! # What is not here
//!
//! A change with no date. An obligation whose start waits on an act not yet
//! adopted has nothing to announce, so it is not an event: an ESPR product group
//! whose delegated act does not exist, the battery data points awaiting an
//! implementing act, and the Art. 8(1) declaration duty, whose stated date is
//! only the earliest it can begin.

mod bundle_origin;
#[cfg(feature = "bundle")]
mod bundle_schedule;
#[cfg(all(test, feature = "bundle"))]
mod bundle_tests;
#[cfg(feature = "bundle")]
mod error;
mod obligation_calendar;
mod obligation_event;
mod obligation_kind;
mod schedule_conflict;
#[cfg(test)]
mod tests;

pub use bundle_origin::BundleOrigin;
#[cfg(feature = "bundle")]
pub use error::ScheduleBundleError;
pub use obligation_calendar::ObligationCalendar;
pub use obligation_event::ObligationEvent;
pub use obligation_kind::ObligationKind;
pub use schedule_conflict::ScheduleConflict;
