//! Electronics spare parts availability rules — EU Ecodesign Regulation (ESPR).
//!
//! ## Current schema state
//! The electronics schema v1.0.0 carries `sparePartsAvailable: bool` — a binary
//! declaration. No availability *period* field is defined yet. There are therefore
//! no cross-field rules to implement until the schema gains a period field.
//!
//! ## ⚠️ COMPLIANCE-PIN PENDING — no period is stated here, and that is the point
//!
//! This module header used to carry a table of minimum spare-parts availability
//! periods — "10 years", "7–10 years" — attributed to Regulations (EU) 2019/2022,
//! 2019/2019 and 2019/2021, with an "(in force)" column beside each.
//!
//! **None of those three acts has been read against its Official Journal text.**
//! The numbers may well be right; nobody here has checked, and the table read
//! authoritatively enough that nobody would have thought to.
//!
//! The table is removed rather than annotated. `CLAUDE.md` is explicit that a
//! secondary-sourced claim may never become a constant, threshold or enumerated
//! category, and this module's own note below says what would have happened
//! next: `validate_spare_parts_period(years, category)` was to be implemented
//! *"using the category-keyed minimum periods above"*. The periods would have
//! become live thresholds at that moment, sourced from a doc comment nobody
//! verified — in a crate published to crates.io, where a release cannot be
//! unpublished.
//!
//! A placeholder is allowed to say *we have not established this*. It is not
//! allowed to assert a number and a source it has not read.
//!
//! ## What is actually known
//!
//! - Minimum spare-parts availability periods are set in **product-specific**
//!   ecodesign implementing regulations, not in the ESPR framework regulation.
//!   Regulations (EU) 2019/2019, 2019/2021 and 2019/2022 are the acts to read;
//!   which periods they set, for which categories, and whether each is still in
//!   force, are all open questions here.
//! - **No ESPR delegated act for an electronics DPP has been adopted.** Under
//!   the ESPR working plan (COM(2025) 187 final) electronics is addressed
//!   through horizontal repairability and EEE-recyclability measures rather than
//!   as a product group.
//! - Regulation (EU) 2023/1670 sets requirements for smartphones and slate
//!   tablets. Not reflected in this module.
//!
//! ## Before implementing anything here
//!
//! Read the three acts, record each period against the article it comes from,
//! and only then write `validate_spare_parts_period(years, category)`. If a
//! period is this project's own figure rather than an act's, it has to say so
//! where a reader will see it — the distinction `ParameterBasis` draws between
//! `Sourced` and `Assumed`, which exists precisely because a number with no
//! stated origin gets read as law.

// Placeholder — rules to be implemented once the electronics schema carries
// a structured spare-parts availability period, the ESPR delegated act
// specifies minimum periods per product category, and the acts above have been
// read. See the module header: the periods this file used to list were never
// verified, and must not be reinstated from it.
