//! [`Gln`] — the GS1 Global Location Number.

mod error;
mod key;
#[cfg(test)]
mod tests;

pub use error::GlnError;
pub use key::Gln;
