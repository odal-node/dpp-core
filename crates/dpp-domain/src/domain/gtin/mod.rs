//! Validated GS1 newtypes (`Gtin`, `Gln`) with GS1 modulo-10 check-digit
//! verification, plus the shared check-digit primitive.

mod check_digit;
mod gln;
#[allow(clippy::module_inception)]
mod gtin;
#[cfg(test)]
mod tests;

pub use check_digit::gs1_check_digit;
pub use gln::{Gln, GlnError};
pub use gtin::{Gtin, GtinError};
