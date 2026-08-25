//! Unsold-goods disclosure lints — Impl. Reg. (EU) 2026/2.
//!
//! A directory module rather than one file because rule 7 of
//! `docs/architecture/CODE-LAYOUT.md` puts tests in a sibling file, which a flat
//! module has nowhere to put. Its two neighbours here still use inline tests and
//! are baselined; this one is new, so it follows the rule.

pub mod lints;

#[cfg(test)]
mod tests;

pub use lints::{DisclosureLineInput, UnsoldGoodsLintInput, lint_unsold_goods};
