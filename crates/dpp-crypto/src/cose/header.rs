//! One header map of a `COSE_Sign1` message: the protected bucket or the
//! unprotected one (RFC 9052 §3).
//!
//! Reading a bucket checks what a bucket alone can break: a label twice, `crit`,
//! and the types of the values this crate reads. The rules that need both buckets
//! are in [`super::sign1`].

use std::collections::BTreeSet;

use super::cbor::{self, Reader};
use super::cwt::{self, CwtClaims};
use super::error::CoseError;

/// Header labels this crate reads (RFC 9052 §3.1, Table 3; RFC 9596 §4.1).
const ALG: i128 = 1;
const CRIT: i128 = 2;
const CONTENT_TYPE: i128 = 3;
const KID: i128 = 4;
const CWT_CLAIMS: i128 = 15;
const TYP: i128 = 16;

/// A header label: an integer or a text string (RFC 9052 §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Label<'a> {
    Integer(i128),
    Text(&'a str),
}

/// The value of an `alg` parameter, which is an integer or a text string
/// (RFC 9052 §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Alg {
    /// An integer that fits an `i64`, the range of every registered algorithm.
    Integer(i64),
    /// A text string, or an integer outside that range: no algorithm of this crate.
    Other,
}

/// The value of a content type (3) or `typ` (16) parameter, borrowed from the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Media<'a> {
    Text(&'a str),
    Unsigned(u64),
}

/// What one header map says about the parameters this crate reads.
#[derive(Debug, Default)]
pub(super) struct Bucket<'a> {
    pub(super) alg: Option<Alg>,
    pub(super) kid: Option<&'a [u8]>,
    pub(super) content_type: Option<Media<'a>>,
    pub(super) typ: Option<Media<'a>>,
    pub(super) cwt_claims: Option<CwtClaims>,
    /// Every label of the map, for the check that no label is in both buckets.
    pub(super) labels: BTreeSet<Label<'a>>,
}

/// Reads the next item as a header map.
///
/// - A label that is neither an integer nor a text string is refused, which
///   RFC 9052 §1.5 permits ("fail processing or ... ignoring"): ignoring one would
///   leave a label this crate cannot compare for duplicates.
/// - A repeated label is refused as malformed (RFC 9052 §3). Labels are compared as
///   values, so `1` written as `0x01` and as `0x1801` is one label. They are kept in
///   a set, so a map of many labels costs no more than `n log n`.
/// - `crit` is refused wherever it is (RFC 9052 §3.1 puts it in the protected
///   header, and this crate understands no critical parameter).
/// - The values of `alg`, content type, `kid` and `typ` must have the types of
///   RFC 9052 §3.1 and RFC 9596 §2, and are refused otherwise. The CWT Claims
///   parameter (15, RFC 9597 §2) must be a map of claims that [`cwt::read_claims`]
///   accepts.
/// - The value of any other label is stepped over unread (`cbor::skip` says what that
///   accepts), as RFC 9052 §3 leaves a recipient free to ignore a parameter it does not
///   know.
pub(super) fn read_bucket<'a>(reader: &mut Reader<'a>) -> Result<Bucket<'a>, CoseError> {
    let pairs = cbor::read_map_len(reader, "a header map")?;
    let mut bucket = Bucket::default();
    for _ in 0..pairs {
        let label = read_label(reader)?;
        if !bucket.labels.insert(label) {
            return Err(CoseError::DuplicateLabel);
        }
        match label {
            Label::Integer(ALG) => bucket.alg = Some(read_alg(reader)?),
            Label::Integer(CRIT) => return Err(CoseError::Critical),
            Label::Integer(CONTENT_TYPE) => {
                bucket.content_type =
                    Some(read_media(reader, "the content type header parameter")?);
            }
            Label::Integer(KID) => {
                bucket.kid = Some(cbor::read_bytes(reader, "the kid header parameter")?);
            }
            Label::Integer(TYP) => {
                bucket.typ = Some(read_media(reader, "the typ header parameter")?);
            }
            Label::Integer(CWT_CLAIMS) => bucket.cwt_claims = Some(cwt::read_claims(reader)?),
            _ => cbor::skip(reader)?,
        }
    }
    Ok(bucket)
}

/// Reads a map key as a label.
fn read_label<'a>(reader: &mut Reader<'a>) -> Result<Label<'a>, CoseError> {
    if cbor::next_is_text(reader)? {
        cbor::read_text(reader, "a header label").map(Label::Text)
    } else if cbor::next_is_unsigned(reader)? || cbor::next_is_negative(reader)? {
        cbor::read_integer(reader, "a header label").map(Label::Integer)
    } else {
        Err(CoseError::Malformed(
            "a header label that is neither an integer nor a text string",
        ))
    }
}

fn read_alg(reader: &mut Reader<'_>) -> Result<Alg, CoseError> {
    if cbor::next_is_text(reader)? {
        cbor::read_text(reader, "the alg header parameter")?;
        return Ok(Alg::Other);
    }
    let value = cbor::read_integer(reader, "the alg header parameter")?;
    Ok(i64::try_from(value).map_or(Alg::Other, Alg::Integer))
}

/// A content type or `typ` value: a text string, or an unsigned integer from the
/// CoAP Content-Formats registry (RFC 9052 §3.1; RFC 9596 §2). A negative integer
/// is refused, since the registry has none, although RFC 9052's CDDL writes `int`.
fn read_media<'a>(reader: &mut Reader<'a>, what: &'static str) -> Result<Media<'a>, CoseError> {
    if cbor::next_is_text(reader)? {
        cbor::read_text(reader, what).map(Media::Text)
    } else {
        cbor::read_unsigned(reader, what).map(Media::Unsigned)
    }
}
