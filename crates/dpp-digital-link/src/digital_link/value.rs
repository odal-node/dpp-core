//! Hold one decoded AI value to the dictionary entry that defines it.
//!
//! The grammar GS1 publishes for the Digital Link path gives every value a
//! length and a character set: `sscc-value` is exactly eighteen digits,
//! `gdti-value` is thirteen digits and then up to seventeen characters of the
//! 82-character set, `cpid-value` draws from the 39-character set. The
//! dictionary says the same thing per AI, in components, so this module reads
//! the rule from there instead of writing it a second time.
//!
//! Five of the dictionary's linters are applied, because all five are
//! mechanical and decided by the value alone: `csum`, the GS1 modulo-10 check
//! digit over the component that names it; `csumalpha`, the check character
//! pair of an alphanumeric key such as a Global Model Number, as the GS1 General
//! Specifications define it; `zero`, which fixes a filler digit at `0`; and
//! `gcppos1` and `gcppos2`, which want the four digits a GS1 Company Prefix
//! begins with at the first or second character, the check GS1's own linter
//! makes. Whether those digits begin a prefix GS1 has actually allocated needs
//! GS1's allocation data, and is not checked. The others — `nozeroprefix`,
//! `pieceoftotal` and the rest — are GS1's deeper validation, and are not run.
//! Nothing here supports a claim of having run them.

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
            "csumalpha" => check_pair(code, part)?,
            "gcppos1" => check_company_prefix(code, part, 0)?,
            "gcppos2" => check_company_prefix(code, part, 1)?,
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

/// The last two characters of `part` are GS1's check character pair over the
/// rest.
///
/// The GS1 General Specifications' "check character calculation (for
/// alphanumeric keys)": each character before the pair is weighted by its
/// position in CSET 82, the weights are multiplied by the primes 2, 3, 5, …
/// counted from the right and summed modulo 1021, and the ten bits of the sum are
/// written as two characters of CSET 32, high five bits first. GS1's syntax
/// engine runs the same calculation, and the oracle holds this one to it.
fn check_pair(code: &str, part: &[char]) -> Result<(), DigitalLinkError> {
    const CSET_82: &str =
        "!\"%&'()*+,-./0123456789:;<=>?ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz";
    const CSET_32: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

    let Some((data, pair)) = part.split_last_chunk::<2>() else {
        // Too short to hold a pair at all, which GS1's linter reports as such.
        return Err(DigitalLinkError::ValueTooShort {
            code: code.to_owned(),
            min_len: 2,
            actual: part.len(),
        });
    };
    let mut primes = (2u32..).filter(|n| (2..*n).take_while(|d| d * d <= *n).all(|d| n % d != 0));
    let mut sum = 0u32;
    for c in data.iter().rev() {
        // The character set was checked first, so every character is in CSET 82.
        let weight = CSET_82.find(*c).map_or(0, |at| at as u32);
        sum = (sum + weight * primes.next().expect("primes do not run out")) % 1021;
    }
    let expected: String = [sum >> 5, sum & 31]
        .iter()
        .map(|five| char::from(CSET_32[*five as usize]))
        .collect();
    let actual: String = pair.iter().collect();
    if expected == actual {
        return Ok(());
    }
    Err(DigitalLinkError::InvalidCheckPair {
        code: code.to_owned(),
        expected,
        actual,
    })
}

/// `part` holds, from `offset` on, the four digits the shortest GS1 Company
/// Prefix has: `gcppos1` reads from the first character and `gcppos2` from the
/// second, after an indicator or extension digit. This is all GS1's linter checks
/// unless it is given GS1's allocation data, which this crate does not have.
fn check_company_prefix(code: &str, part: &[char], offset: usize) -> Result<(), DigitalLinkError> {
    const SHORTEST_PREFIX: usize = 4;
    let digits = part.get(offset..offset + SHORTEST_PREFIX);
    if digits.is_some_and(|digits| digits.iter().all(char::is_ascii_digit)) {
        return Ok(());
    }
    Err(DigitalLinkError::InvalidCompanyPrefix {
        code: code.to_owned(),
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
