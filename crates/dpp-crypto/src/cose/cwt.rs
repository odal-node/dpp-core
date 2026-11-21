//! The CWT Claims header parameter (label 15, RFC 9597 §2): a map of CWT claims
//! (RFC 8392) carried in a COSE header, of which this crate reads the registered
//! claims a verifier acts on.
//!
//! RFC 9597 §2 says it "MUST only occur once in either the protected or unprotected
//! header" and that it is RECOMMENDED "only in a protected header to avoid the contents
//! being malleable", and §1 says recipients "MUST NOT use the information in the CWT
//! Claims header parameter beyond the integrity protection or authentication afforded
//! to the CWT Claims header". A claim in the unprotected header has no integrity
//! protection, so [`super::sign1`] refuses the parameter there and reads it from the
//! protected header only.

use std::collections::BTreeSet;

use super::cbor::{self, Reader};
use super::error::CoseError;
use super::header::Label;

/// The claim keys of RFC 8392 §4, Table 1, that this crate reads.
const ISS: i128 = 1;
const SUB: i128 = 2;
const AUD: i128 = 3;
const EXP: i128 = 4;
const NBF: i128 = 5;
const IAT: i128 = 6;

/// The registered CWT claims of RFC 8392 §3.1 that this crate reads from the CWT Claims
/// header parameter of a verified message: `iss`, `sub`, `aud`, `exp`, `nbf` and `iat`.
///
/// A claim the message does not carry is `None`. Every other claim, `cti` and any
/// private or unregistered one included, is ignored (RFC 8392 §3: "all claims that are
/// not understood by implementations MUST be ignored"). It is stepped over as CBOR, and
/// its key is still held to appearing once.
///
/// What the claims mean for a use of the message is the caller's: RFC 9597 §1 leaves
/// "the intended interpretation" to the application.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct CwtClaims {
    /// The issuer (key 1), a text string.
    pub iss: Option<String>,
    /// The subject (key 2), a text string.
    pub sub: Option<String>,
    /// The audience (key 3): one text string, or an array of text strings
    /// (RFC 8392 §3.1.3). A single string is returned as a list of one.
    pub aud: Option<Vec<String>>,
    /// The expiration time (key 4), a NumericDate in seconds (RFC 8392 §2).
    pub exp: Option<f64>,
    /// The not-before time (key 5), a NumericDate in seconds.
    pub nbf: Option<f64>,
    /// The issued-at time (key 6), a NumericDate in seconds.
    pub iat: Option<f64>,
}

/// Reads the next item as a CWT Claims Set: a definite-length map.
///
/// - A key that is neither an integer nor a text string is refused, and so is a key that
///   appears twice, compared as values, as a header label is (RFC 8392 §3 leaves
///   a repeated key undefined, and a reader that took the first and a reader that took
///   the last would disagree).
/// - The value of `iss` and `sub` must be a text string, `aud` a text string or an array
///   of them, and `exp`, `nbf` and `iat` an integer or a floating-point number that is
///   finite (RFC 8392 §4, Table 1). A value of another type is refused, and so is a
///   tagged one: RFC 8392 §5 says the values of these claims "MUST NOT be prefixed
///   with any CBOR tag".
/// - The value of any other key is stepped over, whatever it is.
pub(super) fn read_claims(reader: &mut Reader<'_>) -> Result<CwtClaims, CoseError> {
    let pairs = cbor::read_map_len(reader, "the CWT Claims header parameter")?;
    let mut claims = CwtClaims::default();
    let mut seen: BTreeSet<Label<'_>> = BTreeSet::new();
    for _ in 0..pairs {
        let key = read_key(reader)?;
        if !seen.insert(key) {
            return Err(CoseError::Malformed("a CWT claim key appears twice"));
        }
        match key {
            Label::Integer(ISS) => claims.iss = Some(read_text(reader, "the iss claim")?),
            Label::Integer(SUB) => claims.sub = Some(read_text(reader, "the sub claim")?),
            Label::Integer(AUD) => claims.aud = Some(read_audience(reader)?),
            Label::Integer(EXP) => claims.exp = Some(read_numeric_date(reader, "the exp claim")?),
            Label::Integer(NBF) => claims.nbf = Some(read_numeric_date(reader, "the nbf claim")?),
            Label::Integer(IAT) => claims.iat = Some(read_numeric_date(reader, "the iat claim")?),
            _ => cbor::skip(reader)?,
        }
    }
    Ok(claims)
}

/// A claim key: an integer or a text string (RFC 8392 §3).
fn read_key<'a>(reader: &mut Reader<'a>) -> Result<Label<'a>, CoseError> {
    if cbor::next_is_text(reader)? {
        cbor::read_text(reader, "a CWT claim key").map(Label::Text)
    } else if cbor::next_is_unsigned(reader)? || cbor::next_is_negative(reader)? {
        cbor::read_integer(reader, "a CWT claim key").map(Label::Integer)
    } else {
        Err(CoseError::Malformed(
            "a CWT claim key that is neither an integer nor a text string",
        ))
    }
}

fn read_text(reader: &mut Reader<'_>, what: &'static str) -> Result<String, CoseError> {
    cbor::read_text(reader, what).map(str::to_owned)
}

/// A text string, or an array of text strings.
fn read_audience(reader: &mut Reader<'_>) -> Result<Vec<String>, CoseError> {
    const WHAT: &str = "the aud claim";
    if cbor::next_is_text(reader)? {
        return Ok(vec![read_text(reader, WHAT)?]);
    }
    let len = cbor::read_array_len(reader, WHAT)?;
    let mut audiences = Vec::new();
    for _ in 0..len {
        audiences.push(read_text(reader, WHAT)?);
    }
    Ok(audiences)
}

/// A finite integer or floating-point number of seconds.
fn read_numeric_date(reader: &mut Reader<'_>, what: &'static str) -> Result<f64, CoseError> {
    let value = if cbor::next_is_float(reader)? {
        cbor::read_float(reader, what)?
    } else {
        // An `i128` read from CBOR's range is exactly an `f64` only up to 2^53, past which
        // a NumericDate is far beyond any date this crate is asked about.
        cbor::read_integer(reader, what)? as f64
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CoseError::Malformed(what))
    }
}
