//! [`Gtin`] — the GS1 trade item number.

mod error;
mod key;
#[cfg(test)]
mod tests;

pub use error::GtinError;
pub use key::Gtin;
