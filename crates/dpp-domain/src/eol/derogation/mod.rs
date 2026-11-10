//! What a destruction claim cites, and the free-text citation older events carry.
//!
//! - [`claim`] — [`Derogation`], a point of the closed list or a legacy citation.
//! - [`legacy`] — [`DerogationRef`], the free text that stays readable.

mod claim;
mod legacy;

pub use claim::Derogation;
pub use legacy::DerogationRef;
