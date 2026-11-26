//! Which battery passport data points a given battery category actually owes.
//!
//! The schema deliberately declares almost every Annex XIII field optional, and
//! that is not laxity: the obligations are **per category**. A field mandatory
//! for an electric-vehicle battery may be *"not to be filled/displayed"* for an
//! LMT one, and requiring it in JSON Schema would make a lawful industrial
//! battery unrepresentable. The constraint belongs here, where it can be
//! expressed as a function of the category.
//!
//! # Source
//!
//! The Commission's *Guidance Document: Digital Batteries Passport — data
//! points by category* (v2.0, 15 August 2026). The table was first read in full
//! against v1.0 (28 July 2026); v2.0 was then compared with v1.0 line by line.
//! Its obligations differ in rows 19 to 23 only, which are taken up below, and
//! the rest of its changes are wording: rows 6 and 25 now give the reason they
//! are not filled, row 35 drops an "only", and a footnote defines "if
//! applicable". Its table has one row per data point and one column per
//! category, which is the shape of the `REQUIREMENTS` table below — so a
//! reviewer can diff the two directly rather than reconstructing the mapping.
//!
//! Version 3.2 (19 October 2026) was compared with this table on 2 November 2026
//! and changes none of its obligations. It splits the C-rate of the cycle-life
//! test, point 1(p), into a mandatory charge C-rate, whose obligations
//! `cycleLifeTestCRate` carries, and a discharge C-rate that is optional for every
//! category and has no field. That renumbers rows between 40 and 50, so the row
//! numbers cited here are 2.0's. It also adds a column for the type of
//! information, which agrees with the battery schema's disclosure classes.
//!
//! v2.0 also states its industrial scope as batteries with a capacity greater
//! than 2 kWh. This module does not see capacity, so it answers for the
//! `industrial` category as a whole.
//!
//! **The guidance covers EV, LMT and industrial batteries only.** It says
//! nothing about portable or SLI batteries, so this module answers
//! [`Requirement::Unknown`] for them rather than guessing. Silence in the source
//! is not permission, and it is not prohibition either.
//!
//! # What this table is not
//!
//! It is **not** a row-for-row copy of the guidance's 71 data points, and the
//! two cannot be diffed on length. This table keys on the wire names of
//! *product group-data* fields, so a guidance row whose content lives on the passport
//! envelope has no row here and is enforced elsewhere:
//!
//! - Data points 3 and 4 (manufacturer name; postal address) are
//!   `manufacturer.name` and `manufacturer.address`, which the domain's own
//!   `Passport::validate` already requires to be non-empty.
//! - Data point 2 (identity of who is registering or is responsible for the
//!   passport) is carried by the responsible-operator and operator-identifier
//!   envelope fields.
//!
//! An omission here is therefore only meaningful for a data point that *does*
//! map to a product group-data field.
//!
//! # Where all 71 rows went
//!
//! Every guidance row has been walked against the model. The difference between
//! 71 and this table's length is accounted for as follows, so a future reader
//! can check the claim rather than take it:
//!
//! - **Envelope, not product group data** — rows 2, 3, 4 (registrant identity;
//!   manufacturer name; postal address), enforced as above. Row 5
//!   (manufacturer web and email) is *"optional, to be filled if such data is
//!   available"*, so it carries no obligation to enforce.
//! - **One block, many rows** — rows 51–60 are the ten members of
//!   `DynamicPerformance`, rows 61–66 the two disjoint `StateOfHealth`
//!   parameter sets, and rows 68–71 the four members of `UsageHistory`. Each
//!   block is one row here because the block is what a passport carries or
//!   omits; the per-member conditions live in the types.
//! - **Deferred by the guidance** — rows 17 to 23 and 44 are *"not to be
//!   filled/displayed as of February 2027"*. The six with a modelled field
//!   (`carbonFootprintClass`, `dueDiligenceUrl`, and the four recycled-content
//!   shares of rows 20 to 23) are listed below as `NotApplicable`; rows 17 and
//!   44 (carbon footprint *declaration*, and instructions for use) have no field
//!   at all, which is the correct state for a data point whose format has not
//!   been specified. `co2ePerUnitKg` is barred beside row 17: it is not the
//!   declaration, but a figure labelled carbon footprint in a passport states one
//!   the guidance says not to display.
//! - **Restated elsewhere in the annex** — row 16 is Annex XIII point 1(c)
//!   material composition, deferred, but its constituents are separately
//!   mandatory as Annex VI Part A points 7, 8 and 10 (rows 12, 13, 15) and are
//!   enforced there.
//!
//! Four omissions were found and closed by this audit's first pass: points 1,
//! 7, 8 and 9 — the unique identifier, model identification, place of
//! manufacture and date of manufacture — all mandatory for every covered
//! category, and none of which any battery schema version had ever declared.
//!
//! # The table is a snapshot; one row moves
//!
//! `REQUIREMENTS` records each obligation as of February 2027, which is what the
//! guidance states, and it stays that way so the two can still be diffed. A row
//! whose answer changes on a date, or turns on facts about the battery, is
//! resolved beside the table rather than in it. There is one: row 19,
//! `dueDiligenceUrl`, which Art. 48(1) makes due from 18 August 2027, subject to
//! Art. 47's two exemptions (see [`super::due_diligence`]). Every function here
//! therefore takes the [`BatteryFacts`](crate::batteries::battery_facts::BatteryFacts) such a row reads, and a row with nothing
//! scheduled ignores them.

mod requirement;
#[cfg(test)]
mod tests;

pub use requirement::*;
