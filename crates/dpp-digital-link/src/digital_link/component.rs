//! One component of an Application Identifier's specification in GS1's
//! dictionary, and the character set it draws from.
//!
//! The dictionary writes an AI's value as components: `N14,csum` is one, and
//! `N13,csum [X..17]` is two, the second optional. Keeping them whole, rather
//! than only the lengths they sum to, is what lets a value be held to each
//! component's own length and character set.

/// The character set a component draws from.
///
/// The letters are GS1's: `N` is numeric, and `X`, `Y` and `Z` are the 82-,
/// 39- and 64-character subsets of ISO/IEC 646 that the GS1 General
/// Specifications define.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharKind {
    /// `N`: the digits.
    Numeric,
    /// `X`: GS1 CSET 82.
    Cset82,
    /// `Y`: GS1 CSET 39, which is the digits, `A` to `Z`, and `-`, `#` and `/`.
    Cset39,
    /// `Z`: GS1 CSET 64, which is the digits, both cases of `A` to `Z`, and `-`,
    /// `_` and `=`.
    Cset64,
}

/// One component of an AI's specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// The character set it draws from.
    pub kind: CharKind,
    /// Fewest characters: the whole length for `N14`, and 1 for `X..20`.
    pub min_len: usize,
    /// Most characters.
    pub max_len: usize,
    /// In square brackets in the dictionary, so it may be absent.
    pub optional: bool,
    /// The linters the entry names for it, such as `csum`, `zero` and
    /// `gcppos1`. Of those only `csum` and `zero` are applied by this crate.
    pub linters: Vec<String>,
}
