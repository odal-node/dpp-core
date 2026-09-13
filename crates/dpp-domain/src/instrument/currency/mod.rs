//! Whether an act is still law, and when anyone last checked.

mod check;
mod state;
#[cfg(test)]
mod tests;

pub use check::CurrencyCheck;
pub use state::CurrencyState;
