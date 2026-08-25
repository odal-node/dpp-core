//! Unsold consumer products — the disclosure required by ESPR Arts. 24–25, in
//! the format its implementing act prescribes.
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
//! # Layout
//!
//! - [`report`] — [`UnsoldGoodsReport`], the whole disclosure.
//! - [`entity`] / [`identifier`] / [`scope`] — who is disclosing, and for whom.
//! - [`financial_year`] — the period, which is the undertaking's own.
//! - [`mod@line`] — one row of the Annex I table, plus [`DiscardedQuantity`].
//!   (Disambiguated: `line` is also `core`'s `line!` macro.)
//! - [`cn_category`] — the CN chapter or heading a line is filed under.
//! - [`reason`] — the Del. Reg. 2026/296 Art. 2 derogations.
//! - [`treatment`] — the six-way percentage split, and the derived total.

pub mod cn_category;
pub mod entity;
pub mod financial_year;
pub mod identifier;
pub mod line;
pub mod reason;
pub mod report;
pub mod scope;
pub mod treatment;

#[cfg(test)]
mod tests;

pub use cn_category::{CnCategory, CnCategoryError};
pub use entity::DisclosingEntity;
pub use financial_year::FinancialYear;
pub use identifier::LegalEntityIdentifier;
pub use line::{DiscardedProductLine, DiscardedQuantity};
pub use reason::DiscardReason;
pub use report::UnsoldGoodsReport;
pub use scope::DisclosureScope;
pub use treatment::WasteTreatmentSplit;
