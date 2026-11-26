//! Battery plausibility lints — physically-grounded consistency checks that
//! EU Battery Regulation 2023/1542 does not itself require, but that flag
//! likely data-entry mistakes: energy/capacity arithmetic, unit-conversion
//! consistency, material-composition sums, and date/range plausibility.

mod checks;
#[cfg(test)]
mod tests;

pub use checks::*;
