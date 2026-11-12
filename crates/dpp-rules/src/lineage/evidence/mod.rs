//! The payload of each kind of evidence a caller can offer for a derivation
//! edge, one per variant of [`Evidence`](super::Evidence).
//!
//! Primitive views rather than domain types, for the reason
//! [`input`](super::input) gives.

pub mod document;
pub mod transfer;
pub mod waste_handover;

pub use document::DocumentEvidence;
pub use transfer::TransferEvidence;
pub use waste_handover::WasteHandoverEvidence;
