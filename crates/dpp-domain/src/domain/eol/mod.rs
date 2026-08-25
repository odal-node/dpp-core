//! End-of-life declarations for a Digital Product Passport.
//!
//! A DPP is never *deleted* at end of life — the passport outlives the product
//! (EN 18221 retention posture). Instead the passport transitions to
//! [`super::status::PassportStatus::Deactivated`] and carries a typed
//! [`EolEvent`] recording *why* and, for circularity (ESPR / Battery Annex XIII),
//! what material was recovered. Destruction specifically must cite a recognised
//! derogation from the unsold-goods destruction ban (ESPR Art. 25 delegated act).

mod deactivation_reason;
#[cfg(test)]
mod deactivation_reason_kinds_tests;
mod derogation_ref;
mod eol_event;
#[cfg(test)]
mod tests;

pub use deactivation_reason::DeactivationReason;
pub use derogation_ref::DerogationRef;
pub use eol_event::EolEvent;
