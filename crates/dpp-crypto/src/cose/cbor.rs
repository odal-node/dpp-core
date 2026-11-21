//! The few CBOR reads and writes `COSE_Sign1` needs, over `cbor4ii`.
//!
//! `cbor4ii` is used as a codec only, with its default features off, which leaves
//! the item-by-item API: the reader borrows from the input and allocates nothing,
//! and a map's length is read and its pairs are then read one at a time, so every
//! key reaches this crate, duplicates included. What each item must be, and what a
//! repeated label means, is decided in [`super::header`] and [`super::sign1`].
//!
//! What the codec does on a read, and what this module adds:
//!
//! - It reads an integer or a length written longer than it needs to be, and so
//!   does this module: RFC 9052 §9 restricts what is *written* for the signed
//!   structure and puts no such limit on reading. The signature is over the
//!   protected header as it arrived, so a longer encoding changes no meaning.
//! - It refuses an indefinite-length byte string or text string where a definite
//!   one is read. For arrays and maps it reports the indefinite form, and this
//!   module refuses it.
//! - Skipping a value it has no use for (`IgnoredAny`) counts nesting against a
//!   limit of 256 levels, which the slice reader sets, and reads nothing it does
//!   not step over. It also steps over some items that RFC 8949 does not call
//!   well formed, such as an indefinite-length string whose chunks are not strings
//!   (§3.2.3), and refuses a simple value other than `false`, `true`, `null`,
//!   `undefined` and the floats. That is acceptable only because a skipped value
//!   is never read: it is covered by the signature in the protected header, and
//!   unused in the unprotected one.

use std::convert::Infallible;

use cbor4ii::core::dec::{Decode, IgnoredAny, Read};
use cbor4ii::core::enc::{self, Encode};
use cbor4ii::core::types;
use cbor4ii::core::utils::SliceReader;

use super::error::CoseError;

/// The reader over a slice, with the library's depth limit.
pub(super) type Reader<'a> = SliceReader<'a>;

/// CBOR major types (RFC 8949 §3.1).
const MAJOR_UNSIGNED: u8 = 0;
const MAJOR_NEGATIVE: u8 = 1;
const MAJOR_BYTES: u8 = 2;
const MAJOR_TEXT: u8 = 3;
const MAJOR_ARRAY: u8 = 4;
const MAJOR_MAP: u8 = 5;
const MAJOR_TAG: u8 = 6;
/// The byte of the simple value `null`, which `COSE_Sign1` writes for a detached
/// payload (RFC 9052 §4.1).
const NULL: u8 = 0xf6;
/// The additional information that marks an indefinite length (RFC 8949 §3.2).
const INDEFINITE: u8 = 0x1f;

/// The major type of an item's first byte.
pub(super) fn major(byte: u8) -> u8 {
    byte >> 5
}

/// The first byte of the next item, not consumed.
pub(super) fn peek(reader: &mut Reader<'_>) -> Result<u8, CoseError> {
    let Ok(available) = reader.fill(1);
    available
        .as_ref()
        .first()
        .copied()
        .ok_or(CoseError::Malformed("the input ends inside an item"))
}

/// Whether nothing is left to read.
pub(super) fn finished(reader: &mut Reader<'_>) -> bool {
    let Ok(available) = reader.fill(1);
    available.as_ref().is_empty()
}

/// Whether the next item is `null`.
pub(super) fn next_is_null(reader: &mut Reader<'_>) -> Result<bool, CoseError> {
    Ok(peek(reader)? == NULL)
}

/// Whether the next item is a text string.
pub(super) fn next_is_text(reader: &mut Reader<'_>) -> Result<bool, CoseError> {
    Ok(major(peek(reader)?) == MAJOR_TEXT)
}

/// Whether the next item is a negative integer.
pub(super) fn next_is_negative(reader: &mut Reader<'_>) -> Result<bool, CoseError> {
    Ok(major(peek(reader)?) == MAJOR_NEGATIVE)
}

/// Whether the next item is an unsigned integer.
pub(super) fn next_is_unsigned(reader: &mut Reader<'_>) -> Result<bool, CoseError> {
    Ok(major(peek(reader)?) == MAJOR_UNSIGNED)
}

/// Whether the next item is a CBOR tag.
pub(super) fn next_is_tag(reader: &mut Reader<'_>) -> Result<bool, CoseError> {
    Ok(major(peek(reader)?) == MAJOR_TAG)
}

/// Reads the number of a CBOR tag.
pub(super) fn read_tag(reader: &mut Reader<'_>) -> Result<u64, CoseError> {
    types::Tag::tag(reader).map_err(|_| CoseError::Malformed("a CBOR tag"))
}

/// Reads an array's header and returns its item count.
pub(super) fn read_array_len(
    reader: &mut Reader<'_>,
    what: &'static str,
) -> Result<usize, CoseError> {
    definite_header(reader, MAJOR_ARRAY, what)?;
    match types::Array::len(reader) {
        Ok(Some(len)) => Ok(len),
        Ok(None) => Err(CoseError::IndefiniteLength),
        Err(_) => Err(CoseError::Malformed(what)),
    }
}

/// Reads a map's header and returns its pair count.
pub(super) fn read_map_len(
    reader: &mut Reader<'_>,
    what: &'static str,
) -> Result<usize, CoseError> {
    definite_header(reader, MAJOR_MAP, what)?;
    match types::Map::len(reader) {
        Ok(Some(len)) => Ok(len),
        Ok(None) => Err(CoseError::IndefiniteLength),
        Err(_) => Err(CoseError::Malformed(what)),
    }
}

/// Checks that the next item has major type `major_type` and is not indefinite.
fn definite_header(
    reader: &mut Reader<'_>,
    major_type: u8,
    what: &'static str,
) -> Result<(), CoseError> {
    let byte = peek(reader)?;
    if major(byte) != major_type {
        return Err(CoseError::Malformed(what));
    }
    if byte & 0x1f == INDEFINITE {
        return Err(CoseError::IndefiniteLength);
    }
    Ok(())
}

/// Reads a definite-length byte string, borrowed from the input.
pub(super) fn read_bytes<'a>(
    reader: &mut Reader<'a>,
    what: &'static str,
) -> Result<&'a [u8], CoseError> {
    definite_header(reader, MAJOR_BYTES, what)?;
    <types::Bytes<&'a [u8]>>::decode(reader)
        .map(|bytes| bytes.0)
        .map_err(|_| CoseError::Malformed(what))
}

/// Reads a definite-length text string of valid UTF-8, borrowed from the input.
pub(super) fn read_text<'a>(
    reader: &mut Reader<'a>,
    what: &'static str,
) -> Result<&'a str, CoseError> {
    definite_header(reader, MAJOR_TEXT, what)?;
    <&'a str>::decode(reader).map_err(|_| CoseError::Malformed(what))
}

/// Reads an unsigned integer.
pub(super) fn read_unsigned(reader: &mut Reader<'_>, what: &'static str) -> Result<u64, CoseError> {
    u64::decode(reader).map_err(|_| CoseError::Malformed(what))
}

/// Reads an integer of either sign, in the range CBOR gives one: `-2^64` to `2^64 - 1`.
pub(super) fn read_integer(reader: &mut Reader<'_>, what: &'static str) -> Result<i128, CoseError> {
    if next_is_unsigned(reader)? {
        read_unsigned(reader, what).map(i128::from)
    } else if next_is_negative(reader)? {
        <types::Negative<u64>>::decode(reader)
            .map(|types::Negative(argument)| -1 - i128::from(argument))
            .map_err(|_| CoseError::Malformed(what))
    } else {
        Err(CoseError::Malformed(what))
    }
}

/// Steps over one item of any kind, nested ones included.
pub(super) fn skip(reader: &mut Reader<'_>) -> Result<(), CoseError> {
    IgnoredAny::decode(reader)
        .map(|IgnoredAny| ())
        .map_err(|_| CoseError::Malformed("a header value that is not well formed CBOR"))
}

// ── Writing ───────────────────────────────────────────────────────────────────

/// An in-memory writer.
#[derive(Default)]
pub(super) struct Out(pub(super) Vec<u8>);

impl enc::Write for Out {
    type Error = Infallible;

    fn push(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.extend_from_slice(input);
        Ok(())
    }
}

/// Writes `value`, mapping the codec's error (which a memory writer cannot raise)
/// onto this module's.
pub(super) fn put(out: &mut Out, value: impl Encode) -> Result<(), CoseError> {
    value.encode(out).map_err(|_| CoseError::Encode)
}

/// Writes the header of a definite-length array of `len` items, with the shortest
/// argument (RFC 8949 §4.2.1).
pub(super) fn put_array_header(out: &mut Out, len: usize) -> Result<(), CoseError> {
    types::Array::bounded(len, out).map_err(|_| CoseError::Encode)
}

/// Writes the header of a definite-length map of `len` pairs, with the shortest
/// argument.
pub(super) fn put_map_header(out: &mut Out, len: usize) -> Result<(), CoseError> {
    types::Map::bounded(len, out).map_err(|_| CoseError::Encode)
}

/// Writes a definite-length byte string, with the shortest length argument.
pub(super) fn put_bytes(out: &mut Out, bytes: &[u8]) -> Result<(), CoseError> {
    put(out, types::Bytes(bytes))
}

/// Writes a definite-length text string, with the shortest length argument.
pub(super) fn put_text(out: &mut Out, text: &str) -> Result<(), CoseError> {
    put(out, text)
}
