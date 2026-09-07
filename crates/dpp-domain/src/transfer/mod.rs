//! Transfer of Responsibility model for EU ESPR DPP.
//!
//! When a product undergoes preparation for reuse, repurposing, or
//! remanufacturing, the new economic operator assumes complete responsibility
//! for providing up-to-date DPP information. The infrastructure must track
//! these transfers with full provenance.
//!
//! The operators a chain names live in [`crate::operator`], a tier below: this
//! module records who has *been* responsible, and the passport states who *is*.
//! Neither owns the concept, so it belongs to neither.
//!
//! ## Module layout
//!
//! - [`record`] — [`TransferRecord`], a single transfer event.
//! - [`status`] — [`TransferStatus`], the transfer state machine's states.
//! - [`chain`] — [`TransferChain`], the append-only transfer history.
//! - [`error`] — [`TransferError`].

pub mod chain;
pub mod error;
pub mod record;
pub mod status;

#[cfg(test)]
mod reason_all_tests;
#[cfg(test)]
mod tests;

pub use chain::TransferChain;
pub use error::TransferError;
pub use record::{TransferReason, TransferRecord};
pub use status::TransferStatus;
