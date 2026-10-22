//! [`ScheduledChange`] — a dated change a rule module knows about, as data.
//!
//! A rule that selects its answer by date holds that date as a constant. Each
//! such module also lists the changes those constants schedule, as a
//! `SCHEDULE` slice built from the **same constants**, so the date a rule
//! applies and the date a calendar announces are one value and cannot disagree.
//!
//! This crate only describes the changes. Asking which of them fall between two
//! dates is a calendar's job, done where the passport-start dates of the
//! instrument catalog can be added to them.
//!
//! It sits under `batteries` because battery rules are the only ones that
//! schedule a dated change today. The type names its product group so that it
//! can move to `common` unchanged when a second group's rules need it.

use crate::common::date::CalendarDate;

/// What kind of obligation a [`ScheduledChange`] brings into effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// A passport data point becomes owed, or stops being barred.
    ContentRequirement,
    /// A minimum share (of recycled content, say) starts to bind.
    MinimumShare,
    /// A substance restriction starts to bind, or a carve-out from one ends.
    Restriction,
}

/// One dated change a rule module schedules.
///
/// Every change listed by this crate is keyed on the date a product is placed
/// on the market, because every rule here takes that date as its input: the
/// change binds products placed on the market on or after [`Self::date`] and
/// reaches none placed earlier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledChange {
    /// Stable identifier, `<product group>.<subject>`, e.g.
    /// `"battery.due-diligence-report"`. Never reused for a different change,
    /// so a consumer that receives an announcement twice can recognise it.
    pub id: &'static str,
    /// The first day the change applies.
    pub date: CalendarDate,
    /// What kind of obligation it is.
    pub kind: ChangeKind,
    /// The provision that fixes the date.
    pub citation: &'static str,
    /// One sentence saying what changes, for a person to read.
    pub summary: &'static str,
    /// The product group's wire key, e.g. `"battery"`.
    pub product_group: &'static str,
    /// The product categories it reaches, as wire names; empty when it reaches
    /// the whole product group.
    pub categories: &'static [&'static str],
}
