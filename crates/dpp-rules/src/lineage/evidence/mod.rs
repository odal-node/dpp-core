//! The payload of each kind of evidence a caller can offer for a derivation
//! edge, one per variant of [`Evidence`](super::Evidence).
//!
//! Primitive views rather than domain types, for the reason the edge itself
//! gives: core and the Wasm plugins each adapt their own representation.

pub mod document;
pub mod transfer;
pub mod waste_handover;

pub use document::DocumentEvidence;
pub use transfer::TransferEvidence;
pub use waste_handover::WasteHandoverEvidence;
