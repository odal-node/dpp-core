//! Hold one decoded AI value to the dictionary entry that defines it.
//!
//! The grammar GS1 publishes for the Digital Link path gives every value a
//! length and a character set: `sscc-value` is exactly eighteen digits,
//! `gdti-value` is thirteen digits and then up to seventeen characters of the
//! 82-character set, `cpid-value` draws from the 39-character set. The
//! dictionary says the same thing per AI, in components, so this module reads
//! the rule from there instead of writing it a second time.
//!
//! Two of the dictionary's linters are applied, because both are mechanical and
//! both are decided by the entry's own text: `csum`, the GS1 modulo-10 check
//! digit over the component that names it, and `zero`, which fixes a filler
//! digit at `0`. The others — `gcppos1` and `gcppos2` (a plausible GS1 Company
//! Prefix), `csumalpha` (the check character pair of a Global Model Number),
//! `nozeroprefix`, `pieceoftotal` and the rest — are GS1's deeper validation,
//! whose reference implementations are a separate resource that is not
//! vendored. Nothing here supports a claim of having run them.

use dpp_domain::gs1_check_digit;
use dpp_rules::common::identifier::is_cset_82;

use super::component::{CharKind, Component};
use super::error::DigitalLinkError;
use super::syntax_dictionary::AiSpec;

/// Hold one decoded AI value to what the dictionary says about that AI.
///
/// The overall length first: GS1 mandates a maximum per AI, and enforcing it
/// keeps an untrusted URI from smuggling an unbounded value downstream, and a
/// minimum, below which the value cannot be the AI at all. Then each component
/// in turn: its characters against its set, and the check digit or filler digit
/// its linters name.
///
/// One function for reading and for building, so the carrier this crate prints
/// is held to exactly the rule its own parser applies.
pub(super) fn check_value(code: &str, spec: &AiSpec, value: &str) -> Result<(), DigitalLinkError> {
    let chars: Vec<char> = value.chars().collect();
    let len = chars.len();
    if len > spec.max_len {
        return Err(DigitalLinkError::ValueTooLong {
            code: code.to_owned(),
            max_len: spec.max_len,
            actual: len,
        });
    }
    if len < spec.min_len {
        return Err(DigitalLinkError::ValueTooShort {
            code: code.to_owned(),
            min_len: spec.min_len,
            actual: len,
        });
    }

    let mut at = 0;
    for component in &spec.components {
        let remaining = len.saturating_sub(at);
        // Only a trailing optional component can be absent: the minimum length
        // above has already required every mandatory one.
        if remaining == 0 {
            break;
        }
        // A fixed-length component takes exactly its length, and the variable
        // one the dictionary allows last takes whatever is left. Clamped to what
        // remains, so a dictionary entry that this reading did not anticipate
        // shortens a component instead of running past the value.
        let take = if component.min_len == component.max_len {
            component.max_len
        } else {
            component.max_len.min(remaining)
        };
        let end = (at + take).min(len);
        let part = &chars[at..end];
        check_charset(code, component.kind, part)?;
        check_linters(code, component, part)?;
        at = end;
    }
    Ok(())
}

/// Every character of `part` belongs to `kind`'s set.
fn check_charset(code: &str, kind: CharKind, part: &[char]) -> Result<(), DigitalLinkError> {
    let (belongs, name): (fn(char) -> bool, &'static str) = match kind {
        CharKind::Cset82 => (is_cset_82, "CSET 82"),
        CharKind::Numeric => (|c| c.is_ascii_digit(), "digits"),
        CharKind::Cset39 => (is_cset_39, "GS1 CSET 39"),
        CharKind::Cset64 => (is_cset_64, "GS1 CSET 64"),
    };
    let Some(&character) = part.iter().find(|c| !belongs(**c)) else {
        return Ok(());
    };
    Err(match kind {
        // Kept as its own error: it predates the other sets, and a serial or
        // lot is where an operator's own characters reach a printed carrier.
        CharKind::Cset82 => DigitalLinkError::OutsideCset82 {
            code: code.to_owned(),
            character,
        },
        _ => DigitalLinkError::OutsideCharset {
            code: code.to_owned(),
            character,
            charset: name,
        },
    })
}

/// The linters of one component that this crate applies.
fn check_linters(code: &str, component: &Component, part: &[char]) -> Result<(), DigitalLinkError> {
    for linter in &component.linters {
        match linter.as_str() {
            "csum" => check_digit(code, part)?,
            "zero" if part.iter().any(|c| *c != '0') => {
                return Err(DigitalLinkError::NonZeroFiller {
                    code: code.to_owned(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

/// The last digit of `part` is the GS1 modulo-10 check digit of the rest.
///
/// `csum` is attached to a component, so it covers that component alone: AI
/// `8003`'s `N1,zero N13,csum` checks the thirteen digits after the filler, not
/// the whole value.
fn check_digit(code: &str, part: &[char]) -> Result<(), DigitalLinkError> {
    // The character set was checked first, so every character is a digit.
    let digits: Vec<u8> = part
        .iter()
        .filter_map(|c| c.to_digit(10))
        .map(|d| d as u8)
        .collect();
    let Some((&actual, data)) = digits.split_last() else {
        return Ok(());
    };
    let expected = gs1_check_digit(data);
    if expected == actual {
        return Ok(());
    }
    Err(DigitalLinkError::InvalidCheckDigit {
        code: code.to_owned(),
        expected,
        actual,
    })
}

/// GS1 CSET 39: the digits, `A` to `Z`, `-`, `#` and `/`.
fn is_cset_39(c: char) -> bool {
    c.is_ascii_digit() || c.is_ascii_uppercase() || matches!(c, '-' | '#' | '/')
}

/// GS1 CSET 64: the digits, both cases of `A` to `Z`, `-`, `_` and `=`.
fn is_cset_64(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '=')
}
