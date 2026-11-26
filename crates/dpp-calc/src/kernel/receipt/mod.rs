//! Proof-of-calculation receipt — auditable envelope for every calculator result.

mod calculation_receipt;
#[cfg(test)]
mod tests;

pub use calculation_receipt::*;
