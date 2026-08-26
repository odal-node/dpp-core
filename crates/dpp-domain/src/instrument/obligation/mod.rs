//! When a passport obligation begins, and on whose authority.

mod date;
mod requirement;
#[cfg(test)]
mod tests;

pub use date::{DateBasis, ObligationDate};
pub use requirement::PassportObligation;
