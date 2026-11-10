//! Unsold consumer products — the disclosure required by ESPR Arts. 24–25, in
//! the format its implementing act prescribes, the discards it summarises, and the
//! documents behind them.
//!
//! # A duty over every product group, not a product group
//!
//! ESPR Art. 24 reaches discarded unsold consumer products generally, and only the
//! Art. 25 destruction ban is limited to the products of Annex VII. The duty is an
//! economic operator's, over a financial year, whatever it sells — so it is not a
//! product group, and the disclosure is not the data of a passport. It has its own
//! module and its own versioned schemas ([`schemas`]).
//!
//! # The two acts this models
//!
//! - **Commission Implementing Regulation (EU) 2026/2** (CELEX `32026R0002`),
//!   made under ESPR Art. 24(3) — the details and format of the disclosure.
//!   Art. 2(1) binds it to **Annex I**; Art. 3 delimits categories by CN code.
//! - **Commission Delegated Regulation (EU) 2026/296** (CELEX `32026R0296`),
//!   made under ESPR Art. 25(5) — the closed list of derogations from the
//!   destruction prohibition, which Annex I note (h) makes the reason vocabulary.
//!
//! Both were adopted on 9 February 2026. The model here predates neither any
//! more.
//!
//! ✅ COMPLIANCE-PIN: Regulation (EU) 2024/1781 Arts. 2(34), 24 and 25 and Annex VII
//! (consolidated 28.6.2024); Impl. Reg. (EU) 2026/2 Arts. 1–4 and Annexes I and II
//! (OJ L, 10.2.2026); Del. Reg. (EU) 2026/296 Arts. 2–4 and 6 (OJ L, 22.4.2026,
//! applying from 19 July 2026). Read against the Official Journal texts.
//!
//! # The disclosure, and what it summarises
//!
//! The **disclosure** is what ESPR Art. 24 has an operator publish each financial
//! year. The **records** are what it summarises: each discard, and the
//! documentation Art. 24(2) and Del. Reg. 2026/296 Art. 3 have the operator keep and
//! hand over on request. Since 19 July 2026 the second part is a condition of
//! destroying an Annex VII product under a derogation, not only a means of
//! checking a disclosure.
//!
//! # Layout
//!
//! The disclosure:
//!
//! - [`report`] — [`UnsoldGoodsReport`], the whole disclosure.
//! - [`entity`] / [`identifier`] / [`scope`] — who is disclosing, and for whom.
//! - [`financial_year`] — the period, which is the undertaking's own.
//! - [`mod@line`] — one row of the Annex I table, plus [`DiscardedQuantity`].
//!   (Disambiguated: `line` is also `core`'s `line!` macro.)
//! - [`reason`] — the Del. Reg. 2026/296 Art. 2 derogations.
//! - [`treatment`] — the six-way percentage split, and the derived total.
//! - [`schemas`] — the disclosure's versioned JSON Schemas, in a registry of their
//!   own.
//!
//! The records:
//!
//! - [`record`] — [`DiscardRecord`], one discard; [`weight`], [`operation`] and
//!   [`receiver`] are the parts of it that are their own types.
//! - [`evidence`] — [`EvidenceKind`] and what each record must be able to show.
//! - [`exemption`] — [`EnterpriseSize`], and who Arts. 24(1) and 25(1) do not reach.
//! - [`retention`] — [`KeepUntil`], the later of the two five-year periods, and
//!   [`Disclosure`], the three states the second one can be in.
//! - [`aggregation`] — [`aggregate`], a year's records into the lines of the
//!   disclosure; [`exclusion`] is what it leaves out and why; `apportion` is how
//!   the treatment shares come to exactly 100.
//!
//! The CN category a line is filed under is
//! [`CnCategory`], which lives in the
//! identifier leaf: it is vocabulary, not unsold-goods data.
//!
//! # What is not here
//!
//! Storing records and documents, enforcing retention, exporting for a competent
//! authority and producing a document within the 30 days Art. 3 allows are a
//! deployment's concern. So is the operator's size class, which decides whether
//! any of this binds it ([`exemption`]): this crate holds neither its headcount nor
//! its turnover, so the host says.

pub mod aggregation;
mod apportion;
pub mod entity;
pub mod evidence;
pub mod exclusion;
pub mod exemption;
pub mod financial_year;
pub mod identifier;
pub mod line;
pub mod operation;
pub mod reason;
pub mod receiver;
pub mod record;
pub mod report;
pub mod retention;
pub mod schemas;
pub mod scope;
pub mod treatment;
pub mod weight;

#[cfg(test)]
mod tests;

pub use crate::identifier::{CnCategory, CnCategoryError};
pub use aggregation::{Aggregation, LineDescriptions, aggregate};
pub use entity::DisclosingEntity;
pub use evidence::{EvidenceKind, EvidenceRequirement, derogation_evidence};
pub use exclusion::{ExcludedRecord, ExclusionReason};
pub use exemption::{EnterpriseSize, MEDIUM_ENTERPRISES_FROM};
pub use financial_year::FinancialYear;
pub use identifier::LegalEntityIdentifier;
pub use line::{DiscardedProductLine, DiscardedQuantity};
pub use operation::WasteTreatmentOperation;
pub use reason::DiscardReason;
pub use receiver::ReceivingOperator;
pub use record::DiscardRecord;
pub use report::UnsoldGoodsReport;
pub use retention::{Disclosure, KeepUntil, keep_until};
pub use schemas::{DISCLOSURE_SCHEMA_KEY, RECORD_SCHEMA_KEY, document_schemas};
pub use scope::DisclosureScope;
pub use treatment::WasteTreatmentSplit;
pub use weight::DiscardedWeight;
