//! Textile plausibility lints — consistency checks the EU textile DPP rules
//! do not themselves require, but that flag likely mistakes or claims made
//! without their usual supporting field.

mod checks;
#[cfg(test)]
mod tests;

pub use checks::*;
