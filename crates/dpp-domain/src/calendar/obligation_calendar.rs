//! [`ObligationCalendar`] — every dated obligation this build knows, in order.

use chrono::NaiveDate;
use dpp_rules::batteries::schedule::ScheduledChange;

use super::{ObligationEvent, ObligationKind};
use crate::instrument::{DateBasis, Instrument, InstrumentCatalog, PassportObligation};

/// Every rule module that schedules a dated change.
///
/// A rule module whose `CalendarDate` constant is not reached from here is a
/// date the calendar cannot see. The drift gate in `dpp-tests` fails on one, so
/// a date added to a rule cannot be forgotten here.
pub(super) const RULE_SCHEDULES: &[&[ScheduledChange]] = &[
    dpp_rules::batteries::due_diligence::SCHEDULE,
    dpp_rules::batteries::recycled_content::SCHEDULE,
    dpp_rules::batteries::chemistry::SCHEDULE,
];

/// Every dated obligation this build knows: the changes the rule modules
/// schedule, and the passport start date of every act in an instrument catalog.
///
/// Built once and then only read. It holds no clock: [`Self::between`] takes
/// both ends of the window, so the same question always has the same answer.
///
/// An act's passport date that is not `YYYY-MM-DD` cannot be placed on a
/// calendar and is left out. The embedded catalog is tested to hold none, so
/// that can only happen to an instrument a caller registers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationCalendar {
    events: Vec<ObligationEvent>,
}

impl ObligationCalendar {
    /// The calendar over the embedded instrument catalog.
    #[must_use]
    pub fn new() -> Self {
        Self::from_catalog(&InstrumentCatalog::new())
    }

    /// The calendar over `catalog`, for a caller that has registered acts of
    /// its own.
    #[must_use]
    pub fn from_catalog(catalog: &InstrumentCatalog) -> Self {
        let mut events: Vec<ObligationEvent> = RULE_SCHEDULES
            .iter()
            .flat_map(|schedule| schedule.iter())
            .filter_map(rule_event)
            .collect();
        for instrument in catalog.all() {
            events.extend(passport_starts(instrument));
        }
        events.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.id.cmp(&b.id)));
        Self { events }
    }

    /// Every event, ordered by date and then by id.
    #[must_use]
    pub fn events(&self) -> &[ObligationEvent] {
        &self.events
    }

    /// The events taking effect from `from` to `to`, both days included, in the
    /// order of [`Self::events`]. Nothing when `from` is after `to`.
    pub fn between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> impl Iterator<Item = &ObligationEvent> + '_ {
        self.events
            .iter()
            .filter(move |e| from <= e.date && e.date <= to)
    }
}

impl Default for ObligationCalendar {
    fn default() -> Self {
        Self::new()
    }
}

/// A rule module's change as an event.
///
/// Its basis is [`DateBasis::Sourced`]: every module that lists a `SCHEDULE`
/// carries a ✅ COMPLIANCE-PIN recording where its dates were read, which the
/// drift gate checks.
fn rule_event(change: &ScheduledChange) -> Option<ObligationEvent> {
    let date = NaiveDate::from_ymd_opt(
        change.date.year,
        u32::from(change.date.month),
        u32::from(change.date.day),
    )?;
    Some(ObligationEvent {
        id: change.id.to_owned(),
        date,
        kind: change.kind.into(),
        citation: change.citation.to_owned(),
        summary: change.summary.to_owned(),
        product_group: Some(change.product_group.to_owned()),
        categories: change.categories.iter().map(|c| (*c).to_owned()).collect(),
        basis: DateBasis::Sourced,
    })
}

/// The passport start dates `instrument` records: one per product-group
/// binding that requires a passport from a date, or one for the act itself when
/// it binds no product group this catalog models.
fn passport_starts(instrument: &Instrument) -> Vec<ObligationEvent> {
    let citation = match &instrument.celex {
        Some(celex) => format!("{} (CELEX {celex})", instrument.title),
        None => instrument.title.clone(),
    };
    let event = |id: String, obligation: &PassportObligation, product_group: Option<&str>| {
        let from = obligation.applies_from()?;
        let date = NaiveDate::parse_from_str(&from.date, "%Y-%m-%d").ok()?;
        Some(ObligationEvent {
            id,
            date,
            kind: ObligationKind::PassportStart,
            citation: citation.clone(),
            summary: format!(
                "{}: a digital product passport is required from this date, for the \
                 products its scope reaches.",
                instrument.title
            ),
            product_group: product_group.map(str::to_owned),
            categories: Vec::new(),
            basis: from.basis,
        })
    };

    if instrument.product_groups.is_empty() {
        return event(
            format!("{}.passport", instrument.id),
            &instrument.passport,
            None,
        )
        .into_iter()
        .collect();
    }
    instrument
        .product_groups
        .iter()
        .filter_map(|binding| {
            event(
                format!("{}.passport.{}", instrument.id, binding.product_group),
                binding.passport.as_ref().unwrap_or(&instrument.passport),
                Some(&binding.product_group),
            )
        })
        .collect()
}
