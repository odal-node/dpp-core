//! The suite's disclosure paths, and the `ClaimPath`s `dpp-vc` takes.
//!
//! The suite writes a path as a dotted string, with `[n]` after a member for one array
//! element: `credentialSubject.phoneNumbers[0]`. The library takes `ClaimPath`s and has no
//! string syntax of its own, so the conversion is this harness's. A member name that holds
//! a `.`, `[` or `]` cannot be written in the suite's syntax, and none of its paths has one.

mod claim_path;
#[cfg(test)]
mod tests;

pub use claim_path::*;
