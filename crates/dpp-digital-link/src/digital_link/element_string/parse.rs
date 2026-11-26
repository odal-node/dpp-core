//! Parsing of a GS1 element string into its application identifiers.

use dpp_domain::Gtin;

use super::super::codec::normalize_gtin_to_14;
use super::super::error::DigitalLinkError;
use super::super::link::DigitalLink;
use super::super::syntax_dictionary::{ai_len_for_prefix, ai_spec};

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
            primary_key: super::super::primary_key::PrimaryKey::Gtin(gtin),
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
) -> Result<(&str, &'static super::super::syntax_dictionary::AiSpec), DigitalLinkError> {
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
