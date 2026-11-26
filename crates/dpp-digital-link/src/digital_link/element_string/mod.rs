//! [`ElementString`] — the AI data a barcode scanner actually emits.
//!
//! A Digital Link URI is one representation of GS1 AI data. The other is the
//! *element string*: AIs and their values concatenated, with variable-length
//! values terminated by FNC1 (`\x1D` as read from a scanner), optionally behind
//! a symbology identifier such as `]d2`.
//!
//! ```text
//! ]d201095060001343522112345\x1D10BATCH01
//!  ^^^ symbology   ^^ AI 01, 14 digits (pre-defined length, no separator)
//!                                 ^^ AI 21, variable, ends at FNC1
//!                                                ^^ AI 10, variable, ends at input
//! ```
//!
//! Which AIs are pre-defined length comes from GS1's published Syntax
//! Dictionary — see [`super::syntax_dictionary`]. It is never hard-coded here:
//! one wrong fixed-vs-variable decision silently truncates or over-reads a
//! value scanned off a physical product, and the result still looks like a
//! plausible identifier.

mod parse;
#[cfg(test)]
mod tests;

pub use parse::*;
