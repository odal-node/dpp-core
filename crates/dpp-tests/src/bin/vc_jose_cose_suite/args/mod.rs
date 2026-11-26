//! The command line the suite builds, in `tests/test-util.js`:
//!
//! ```text
//! <command> --input <file> --key <file> --feature <name> [--sd <json>] --output <file>
//! ```

mod parse;
#[cfg(test)]
mod tests;

pub use parse::*;
