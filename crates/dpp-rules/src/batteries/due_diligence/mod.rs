//! Whether a battery's passport owes the due diligence report — Regulation (EU)
//! 2023/1542 **Art. 47 and Art. 48(1)**, reached through Annex XIII point 1(d).
//!
//! ✅ COMPLIANCE-PIN: EU 2023/1542, Art. 47 and Art. 48(1) as replaced by
//! Regulation (EU) 2025/1561, read verbatim from the consolidated text
//! `02023R1542 -- EN -- 31.07.2025 -- 002.004`, p. 45, on 2026-10-21. The date
//! is the article's; the turnover figure is quoted below and deliberately not
//! encoded anywhere.
//!
//! Art. 48(1):
//!
//! > "From 18 August 2027, economic operators that place batteries on the market
//! > or put them into service shall fulfil the due diligence obligations laid
//! > down in paragraphs 2 and 3 of this Article, and in Articles 49, 50 and 52
//! > and shall, to that end, set up and implement battery due diligence
//! > policies."
//!
//! Annex XIII point 1(d) puts in the passport *"information on responsible
//! sourcing as indicated in the report on battery due diligence policy referred
//! to in Article 52(3)"*. The field is the report's address, so it is owed
//! exactly where the report is.
//!
//! ## Art. 47 takes two cases out of the chapter
//!
//! Art. 47, first and second paragraphs, same page:
//!
//! > "This Chapter does not apply to economic operators that had a net turnover
//! > of less than EUR 40 million in the financial year preceding the last
//! > financial year, and that are not part of a group, consisting of parent and
//! > subsidiary undertakings, which, on a consolidated basis, exceeds the limit
//! > of EUR 40 million."
//!
//! > "This Chapter does not apply to economic operators in relation to the
//! > placing on the market or putting into service of batteries that have been
//! > subject to preparation for re-use, preparation for repurposing, repurposing
//! > or remanufacturing, if such batteries had already been placed on the market
//! > or put into service before undergoing such operations."
//!
//! Neither case has a report for the field to point at. The Commission's
//! data-point guidance (v2.0, row 19) says only *"as provided in Article 48(1),
//! required from August 2027"* and does not mention Art. 47. It also says it
//! introduces no requirement of its own, so it cannot be read as setting the
//! article aside.
//!
//! ## Why the turnover limb is declared, not computed
//!
//! The first paragraph turns on a figure about the operator in a past financial
//! year. A passport cannot carry that figure honestly, and this crate cannot
//! check it. So the operator **declares** which side of the threshold it is on
//! ([`OperatorScope`]), and the amount never appears in code. If the threshold
//! is amended, a citation changes and no branch does.
//!
//! An undeclared scope is **not** an exemption. Reading the absence as "below
//! the threshold" would excuse every operator that left the field out.
//!
//! ## Which date governs
//!
//! The placing-on-market (or putting-into-service) date, never the date of
//! assessment. Art. 48(1) attaches the duty to the act of placing, so a battery
//! placed before 18 August 2027 was placed by an operator not then bound, and
//! does not acquire the duty by being reassessed later. This is the same reading
//! [`art8_phase_for`](super::recycled_content::art8_phase_for) applies to Art. 8.

mod operator_scope;
mod scope;
mod second_life;
#[cfg(test)]
mod tests;

pub use operator_scope::OperatorScope;
pub use scope::{DUE_DILIGENCE_REQUIRED_FROM, DueDiligenceScope, SCHEDULE, due_diligence_scope};
pub use second_life::DueDiligenceSecondLife;
