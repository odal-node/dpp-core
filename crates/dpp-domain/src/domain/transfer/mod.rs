//! Transfer of Responsibility model for EU ESPR DPP.
//!
//! When a product undergoes preparation for reuse, repurposing, or
//! remanufacturing, the new economic operator assumes complete responsibility
//! for providing up-to-date DPP information. The infrastructure must track
//! these transfers with full provenance.
//!
//! ## Module layout
//!
//! - [`operator`] — [`ResponsibleOperator`] and its [`OperatorRole`].
//! - [`record`] — [`TransferRecord`], a single transfer event.
//! - [`status`] — [`TransferStatus`], the transfer state machine's states.
//! - [`chain`] — [`TransferChain`], the append-only transfer history.
//! - [`error`] — [`TransferError`].

pub mod chain;
pub mod error;
pub mod operator;
pub mod record;
pub mod status;

#[cfg(test)]
mod tests;

pub use chain::TransferChain;
pub use error::TransferError;
pub use operator::{OperatorRole, ResponsibleOperator};
pub use record::{TransferReason, TransferRecord};
pub use status::TransferStatus;
