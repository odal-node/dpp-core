//! End-of-life declarations for a Digital Product Passport.
//!
//! A DPP is never *deleted* at end of life — the passport outlives the product
//! (EN 18221 retention posture). Instead the passport transitions to
//! [`super::status::PassportStatus::Deactivated`] and carries a typed
//! [`EolEvent`] recording *why* and, for circularity (ESPR / Battery Annex XIII),
//! what material was recovered. A destruction the unsold-goods destruction ban
//! (ESPR Art. 25(1)) reaches must cite a derogation from it, a point of the closed
//! list of Commission Delegated Regulation (EU) 2026/296 Art. 2; one it does not
//! reach needs none. See [`EolEvent::missing_derogation`].

mod deactivation_reason;
#[cfg(test)]
mod deactivation_reason_kinds_tests;
mod derogation;
mod event;
#[cfg(test)]
mod tests;

pub use deactivation_reason::DeactivationReason;
pub use derogation::{Derogation, DerogationRef};
pub use event::EolEvent;
