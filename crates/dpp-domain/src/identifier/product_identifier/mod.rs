//! The unique product identifier, across the EN 18219 clause 5 schemes.
//!
//! One type per scheme would leave every caller matching on which one it holds.
//! One enum leaves the choice where the standard puts it — clause 5.1 says an
//! identifier complies with *one of* the schemes, and they are alternatives.

mod error;
mod scheme;
#[cfg(test)]
mod tests;

pub use error::ProductIdentifierError;
pub use scheme::ProductIdentifier;
