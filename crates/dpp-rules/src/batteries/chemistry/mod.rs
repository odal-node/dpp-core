//! Battery substance restrictions — Regulation (EU) 2023/1542 **Art. 6** and
//! **Annex I**.
//!
//! Art. 6(1) is the operative prohibition: batteries shall not contain
//! substances for which Annex I sets a restriction, other than under the
//! conditions Annex I states. The thresholds themselves are in Annex I, which
//! has exactly three entries.
//!
//! This module cited **Art. 9** for all of it until 2026-08-27. Art. 9 is
//! *Performance and durability requirements for portable batteries of general
//! use* and says nothing about substances. The thresholds were right and the
//! citation was not, which is the failure mode that survives longest: checking
//! it requires opening the act.
//!
//! ## Annex I entry 1 — mercury, all battery types
//!
//! *"Batteries, whether or not incorporated into appliances, light means of
//! transport or other vehicles, shall not contain more than 0,0005 % of mercury
//! (expressed as mercury metal) by weight."* No date gate — in force now.
//!
//! ## Annex I entry 2 — cadmium, portable batteries only
//!
//! *"Portable batteries … shall not contain more than 0,002 % of cadmium
//! (expressed as cadmium metal) by weight."* No date gate. Industrial and EV
//! batteries carry no cadmium weight-percentage limit.
//!
//! ## Annex I entry 3 — lead, portable batteries, dated
//!
//! *"From 18 August 2024, portable batteries, whether or not incorporated into
//! appliances, shall not contain more than 0,01 % of lead (expressed as lead
//! metal) by weight"*, and *"the restriction … shall not apply to portable
//! zinc-air button cells until 18 August 2028"*.
//!
//! Alone among the three it has both a start date and a carve-out, so it is the
//! only one whose answer depends on when the battery was placed on the market
//! and what kind of cell it is. It was also the one missing entirely from this
//! module while its two siblings were implemented.
//!
//! ## Operating temperature range (cross-field)
//! If both `operatingTempMinC` and `operatingTempMaxC` are declared in the
//! battery DPP, the minimum must be strictly less than the maximum. JSON Schema
//! cannot express this comparison across two fields.

mod limits;
#[cfg(test)]
mod tests;

pub use limits::*;
