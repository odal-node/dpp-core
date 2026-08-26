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

use dpp_domain::Gtin;

use super::codec::normalize_gtin_to_14;
use super::error::DigitalLinkError;
use super::link::DigitalLink;
use super::syntax_dictionary::{ai_len_for_prefix, ai_spec};

/// The FNC1 separator, as a scanner transmits it.
const FNC1: char = '\u{1D}';

/// GS1 AI data decoded from a scanned element string.
///
/// The typed fields are the AIs this crate models in a Digital Link path. Every
/// other AI is kept in [`ElementString::other`], in the order it was read —
/// data scanned off a physical product is never silently dropped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ElementString {
    /// The symbology identifier that prefixed the data, e.g. `"]d2"`, if present.
    pub symbology: Option<String>,
    /// AI 01, normalised to 14 digits and check-digit validated.
    pub gtin: Option<Gtin>,
    /// AI 22 — consumer product variant.
    pub variant: Option<String>,
    /// AI 10 — batch/lot.
    pub batch: Option<String>,
    /// AI 21 — serial.
    pub serial: Option<String>,
    /// AI 235 — third-party controlled serial.
    pub tpcsn: Option<String>,
    /// Every AI not modelled above, as `(ai, value)`, in the order read.
    pub other: Vec<(String, String)>,
}

impl ElementString {
    /// Parse a scanned element string.
    ///
    /// Accepts an optional leading symbology identifier (`]` plus two
    /// characters). Values of pre-defined-length AIs are taken at their exact
    /// length; variable-length values run to the next FNC1 or the end of input.
    ///
    /// # Errors
    ///
    /// - [`DigitalLinkError::UnknownApplicationIdentifier`] — no AI in GS1's
    ///   dictionary matches at this position. Parsing stops rather than
    ///   guessing, because a mis-split shifts every subsequent value.
    /// - [`DigitalLinkError::TrailingUnpairedSegment`] — an AI with no value.
    /// - [`DigitalLinkError::ValueTooLong`] — a value longer than the AI permits.
    /// - GTIN errors, when AI 01 is present and malformed.
    pub fn parse(input: &str) -> Result<Self, DigitalLinkError> {
        let (symbology, mut rest) = split_symbology(input);
        let mut out = ElementString {
            symbology: symbology.map(str::to_owned),
            ..Default::default()
        };

        while !rest.is_empty() {
            // A separator may precede the next AI when the previous value was
            // pre-defined length and the encoder emitted one anyway.
            rest = rest.trim_start_matches(FNC1);
            if rest.is_empty() {
                break;
            }

            let (ai, spec) = match_ai(rest)?;
            rest = &rest[ai.len()..];

            let value_len = if spec.predefined_length {
                if rest.len() < spec.max_len {
                    return Err(DigitalLinkError::TrailingUnpairedSegment(ai.to_owned()));
                }
                spec.max_len
            } else {
                rest.find(FNC1).unwrap_or(rest.len())
            };

            let value = &rest[..value_len];
            rest = &rest[value_len..];

            if value.is_empty() {
                return Err(DigitalLinkError::TrailingUnpairedSegment(ai.to_owned()));
            }
            if value.len() > spec.max_len {
                return Err(DigitalLinkError::ValueTooLong {
                    code: ai.to_owned(),
                    max_len: spec.max_len,
                    actual: value.len(),
                });
            }

            out.assign(ai, value)?;
        }

        Ok(out)
    }

    /// Route one decoded AI to its typed field, or to `other`.
    fn assign(&mut self, ai: &str, value: &str) -> Result<(), DigitalLinkError> {
        match ai {
            "01" => self.gtin = Some(Gtin::parse(&normalize_gtin_to_14(value)?)?),
            "22" => self.variant = Some(value.to_owned()),
            "10" => self.batch = Some(value.to_owned()),
            "21" => self.serial = Some(value.to_owned()),
            "235" => self.tpcsn = Some(value.to_owned()),
            _ => self.other.push((ai.to_owned(), value.to_owned())),
        }
        Ok(())
    }

    /// Build a Digital Link URI on `resolver_base` from the decoded AIs.
    ///
    /// # Errors
    ///
    /// [`DigitalLinkError::MissingGtin`] when no AI 01 was present.
    ///
    /// GS1 defines sixteen Digital Link primary keys and [`DigitalLink`] now
    /// reads all of them, but this type models the GTIN family of AIs as typed
    /// fields, so an element string keyed on another primary key keeps that key
    /// in [`ElementString::other`] and cannot be lifted to a link from here.
    /// Parsing a *URI* on any primary key is unaffected.
    pub fn to_digital_link(&self, resolver_base: &str) -> Result<DigitalLink, DigitalLinkError> {
        let gtin = self.gtin.clone().ok_or(DigitalLinkError::MissingGtin)?;

        // Emitted in GS1's canonical qualifier order for AI 01, which is the
        // order `dlpkey=22,10,21|235` declares.
        let mut qualifiers = Vec::new();
        for (ai, value) in [
            ("22", &self.variant),
            ("10", &self.batch),
            ("21", &self.serial),
            ("235", &self.tpcsn),
        ] {
            if let Some(v) = value {
                qualifiers.push((ai.to_owned(), v.clone()));
            }
        }

        Ok(DigitalLink {
            resolver_base: resolver_base.trim_end_matches('/').to_owned(),
            primary_key: super::primary_key::PrimaryKey::Gtin(gtin),
            qualifiers,
        })
    }
}

/// Split a leading symbology identifier (`]` plus two characters) from the data.
fn split_symbology(input: &str) -> (Option<&str>, &str) {
    if input.starts_with(']') && input.len() >= 3 && input.is_char_boundary(3) {
        (Some(&input[..3]), &input[3..])
    } else {
        (None, input)
    }
}

/// Read the AI at the start of `rest`, using GS1's rule that the leading two
/// digits determine the AI's length.
///
/// Deterministic by construction, which longest-match is not: an AI followed by
/// a value whose first digits happen to complete a longer AI would be
/// mis-split, and every value after it would shift.
fn match_ai(
    rest: &str,
) -> Result<(&str, &'static super::syntax_dictionary::AiSpec), DigitalLinkError> {
    // Report a bounded prefix on failure: the remainder of the buffer is not an
    // identifier, and echoing all of it into an error message is noise.
    let unknown =
        || DigitalLinkError::UnknownApplicationIdentifier(rest.chars().take(4).collect::<String>());

    let prefix = rest.get(..2).ok_or_else(unknown)?;
    let len = ai_len_for_prefix(prefix).ok_or_else(unknown)?;
    let ai = rest.get(..len).ok_or_else(unknown)?;
    let spec = ai_spec(ai).ok_or_else(unknown)?;
    Ok((ai, spec))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A valid GTIN-14 used throughout; its check digit is correct.
    const GTIN: &str = "09506000134352";

    #[test]
    fn reads_a_predefined_length_ai_without_a_separator() {
        let parsed = ElementString::parse(&format!("01{GTIN}")).expect("parses");
        assert_eq!(parsed.gtin.map(|g| g.to_string()), Some(GTIN.to_owned()));
        assert!(parsed.other.is_empty());
    }

    /// The realistic scan: symbology identifier, a pre-defined-length GTIN with
    /// no separator after it, a variable serial ended by FNC1, and a variable
    /// batch ended by the input.
    #[test]
    fn reads_a_full_scan() {
        let parsed =
            ElementString::parse(&format!("]d201{GTIN}2112345\u{1D}10BATCH01")).expect("parses");

        assert_eq!(parsed.symbology.as_deref(), Some("]d2"));
        assert_eq!(parsed.gtin.map(|g| g.to_string()), Some(GTIN.to_owned()));
        assert_eq!(parsed.serial.as_deref(), Some("12345"));
        assert_eq!(parsed.batch.as_deref(), Some("BATCH01"));
    }

    /// The whole point of reading lengths from GS1's dictionary. `21` is
    /// variable, so its value runs to the separator; a parser that treated it as
    /// fixed would truncate the serial and read the remainder as an AI.
    #[test]
    fn a_variable_value_runs_to_the_separator_not_a_fixed_width() {
        let parsed =
            ElementString::parse(&format!("01{GTIN}21SN-0000000001\u{1D}10B1")).expect("parses");
        assert_eq!(parsed.serial.as_deref(), Some("SN-0000000001"));
        assert_eq!(parsed.batch.as_deref(), Some("B1"));
    }

    /// An AI this crate does not model as a typed field is still returned.
    /// Silently dropping data read off a physical product is the dangerous
    /// failure, not the noisy one.
    #[test]
    fn keeps_unmodelled_ais_rather_than_dropping_them() {
        // 17 is a pre-defined-length date (N6), so no separator follows it.
        let parsed = ElementString::parse(&format!("01{GTIN}17260821")).expect("parses");
        assert_eq!(parsed.other, vec![("17".to_owned(), "260821".to_owned())]);
    }

    /// Four-digit AIs are read as four digits, from the same prefix rule.
    #[test]
    fn reads_a_four_digit_ai() {
        let parsed = ElementString::parse(&format!("01{GTIN}3103000123")).expect("parses");
        assert_eq!(parsed.other, vec![("3103".to_owned(), "000123".to_owned())]);
    }

    #[test]
    fn rejects_an_unassigned_ai() {
        let err = ElementString::parse("05SOMETHING").expect_err("05 is unassigned");
        assert!(
            matches!(err, DigitalLinkError::UnknownApplicationIdentifier(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_a_truncated_predefined_length_value() {
        let err = ElementString::parse("010950600013").expect_err("GTIN is short");
        assert!(
            matches!(err, DigitalLinkError::TrailingUnpairedSegment(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_an_over_long_variable_value() {
        // AI 21 permits at most 20 characters.
        let err = ElementString::parse(&format!("01{GTIN}21{}", "X".repeat(21)))
            .expect_err("serial too long");
        assert!(
            matches!(err, DigitalLinkError::ValueTooLong { max_len: 20, .. }),
            "got {err:?}"
        );
    }

    /// A bad check digit is caught here, not carried forward.
    #[test]
    fn validates_the_gtin_check_digit() {
        let err = ElementString::parse("0109506000134353").expect_err("check digit is wrong");
        assert!(
            matches!(err, DigitalLinkError::InvalidGtinCheckDigit { .. }),
            "got {err:?}"
        );
    }

    /// The conversion callers actually want, and it round-trips through the
    /// URI parser this crate already had.
    #[test]
    fn converts_to_a_digital_link_and_round_trips() {
        let parsed =
            ElementString::parse(&format!("]d201{GTIN}2112345\u{1D}10BATCH01")).expect("parses");
        let link = parsed
            .to_digital_link("https://id.example.com")
            .expect("has a GTIN");

        let uri = link.build();
        let reparsed = DigitalLink::parse(&uri).expect("round-trips");
        assert_eq!(reparsed.gtin().unwrap().to_string(), GTIN);
        assert_eq!(reparsed.serial(), Some("12345"));
        assert_eq!(reparsed.batch(), Some("BATCH01"));
    }

    /// GS1 defines sixteen Digital Link primary keys and this crate builds
    /// links only on GTIN. An element string keyed on another one still
    /// *decodes* — the data is not lost — but cannot become a link yet.
    #[test]
    fn decodes_a_non_gtin_primary_key_but_cannot_build_a_link() {
        // AI 8003 (GRAI): a leading zero, a 13-digit key, then an optional
        // serial — variable length overall, so the value runs to the end.
        //
        // Note the input must not be written as `8003` + `0` + the key by
        // accident: `8030` is itself an assigned AI, and the prefix rule would
        // correctly read that instead. The AI here is the first four characters.
        let parsed = ElementString::parse("800309506000134352ASSET1").expect("decodes");
        assert_eq!(
            parsed.other,
            vec![("8003".to_owned(), "09506000134352ASSET1".to_owned())]
        );

        let err = parsed
            .to_digital_link("https://id.example.com")
            .expect_err("no GTIN present");
        assert!(matches!(err, DigitalLinkError::MissingGtin), "got {err:?}");
    }
}
