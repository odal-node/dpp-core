//! The parts of a `data:` URL (RFC 2397) that an Enveloped Verifiable Credential is read
//! by, and the decoding of its data.
//!
//! **Reading is more forgiving than writing.** VC-JOSE-COSE §3.3.1 says a COSE credential
//! in a presentation "MUST be encoded using base64 as specified in [RFC2397]". RFC 2397 says
//! that the appearance of `;base64` "means that the data is encoded as base64", and defines
//! no more of it. The base64 of the MIME specification it cites for the tokens of a media
//! type pads: RFC 2045 §6.8 says "A full encoding quantum is always completed at the end of a
//! body". The MUST binds the encoder, and an encoder writes padded standard base64 with no
//! whitespace. A reader is held to nothing by it, and the W3C test suite's own presentation
//! (`presentation-jose-multiple.txt`, row 7) carries a `vc+cose` envelope whose base64 has no
//! padding and ends in a line feed. So the data of a `;base64` URL is read with the WHATWG
//! Infra Standard's "forgiving-base64 decode", which [`forgiving_base64_decode`] implements.

/// The parts of a `data:` URL that an envelope is read by.
pub(super) struct DataUrl<'a> {
    /// The media type, lower-cased and without its parameters.
    pub(super) media_type: String,
    /// Whether the parameters end in `base64`.
    pub(super) base64: bool,
    /// What follows the comma, still encoded.
    pub(super) data: &'a str,
}

/// Split a `data:` URL, `data:[<mediatype>][;base64],<data>`. The scheme is not
/// case-sensitive, and an empty media type means `text/plain`. Only the part
/// before the comma is read.
pub(super) fn split_data_url(id: &str) -> Option<DataUrl<'_>> {
    let rest = id
        .get(..5)
        .filter(|scheme| scheme.eq_ignore_ascii_case("data:"))
        .map(|_| &id[5..])?;
    let (meta, data) = rest.split_once(',')?;
    let mut parameters = meta.split(';');
    let media_type = parameters.next().unwrap_or_default();
    let media_type = if media_type.is_empty() {
        "text/plain".to_owned()
    } else {
        media_type.to_ascii_lowercase()
    };
    Some(DataUrl {
        media_type,
        base64: parameters.next_back() == Some("base64"),
        data,
    })
}

/// The data of a `data:` URL as bytes: percent-decoded, and decoded from base64 as
/// [`forgiving_base64_decode`] reads it when the URL says `;base64`. The error says why the
/// data cannot be read, and carries none of it.
pub(super) fn decode_data(data: &str, base64: bool) -> Result<Vec<u8>, &'static str> {
    let bytes = percent_decode(data).ok_or("a percent escape that is not two hex digits")?;
    if !base64 {
        return Ok(bytes);
    }
    let text = String::from_utf8(bytes).map_err(|_| "base64 data that is not text")?;
    forgiving_base64_decode(&text).ok_or("data that is not base64")
}

/// The data of a `data:` URL as text, decoded as [`decode_data`] decodes it.
pub(super) fn decode_text(data: &str, base64: bool) -> Result<String, &'static str> {
    String::from_utf8(decode_data(data, base64)?).map_err(|_| "data that is not UTF-8 text")
}

/// `data` with its `%XX` escapes decoded, or `None` when an escape is not two hex
/// digits.
fn percent_decode(data: &str) -> Option<Vec<u8>> {
    let bytes = data.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // Two hex digits (RFC 3986 §2.1), checked before parsing: `from_str_radix` also
            // reads a leading `+`.
            let hex = data.get(i + 1..i + 3)?;
            if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(out)
}

/// The WHATWG Infra Standard's "forgiving-base64 decode" (<https://infra.spec.whatwg.org/>,
/// section 7, "Forgiving base64"), which the standard introduces as "different from the RFC
/// as it defines error handling for certain inputs". Given a string `data`, in the
/// standard's order:
///
/// 1. "Remove all ASCII whitespace from data": U+0009, U+000A, U+000C, U+000D and U+0020.
/// 2. "If data's code point length divides by 4 leaving no remainder", and it "ends with one
///    or two U+003D (=) code points, then remove them".
/// 3. "If data's code point length divides by 4 leaving a remainder of 1, then return
///    failure."
/// 4. "If data contains a code point that is not one of U+002B (+), U+002F (/), [or] ASCII
///    alphanumeric then return failure."
/// 5. Each remaining code point is six bits of the standard alphabet of RFC 4648 §4, most
///    significant bit first. Every 24 bits are three bytes, and a final 12 or 18 bits give
///    one or two bytes, the last four or two bits discarded. So `YQ` and `YR` both decode
///    to `a`.
///
/// Anything else is `None`. This is looser than RFC 2045's base64, which needs the final
/// quantum completed with `=` and takes no byte that is not in its alphabet or whitespace.
/// The URL alphabet of RFC 4648 §5 (`-` and `_`) is not accepted, and `=` is accepted only
/// as the one or two characters that end a text whose length is a multiple of four.
pub(crate) fn forgiving_base64_decode(data: &str) -> Option<Vec<u8>> {
    let mut symbols: Vec<char> = data
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\u{000C}' | '\r' | ' '))
        .collect();
    if symbols.len().is_multiple_of(4) {
        for _ in 0..2 {
            if symbols.last() == Some(&'=') {
                symbols.pop();
            }
        }
    }
    if symbols.len() % 4 == 1 {
        return None;
    }
    let mut output = Vec::with_capacity(symbols.len() / 4 * 3 + 2);
    let mut buffer: u32 = 0;
    let mut bits = 0_u32;
    for symbol in symbols {
        let value = match symbol {
            'A'..='Z' => symbol as u32 - 'A' as u32,
            'a'..='z' => symbol as u32 - 'a' as u32 + 26,
            '0'..='9' => symbol as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits == 24 {
            output.extend_from_slice(&buffer.to_be_bytes()[1..]);
            buffer = 0;
            bits = 0;
        }
    }
    match bits {
        0 => {}
        12 => output.push(u8::try_from(buffer >> 4).ok()?),
        18 => {
            let value = buffer >> 2;
            output.push(u8::try_from(value >> 8).ok()?);
            output.push(u8::try_from(value & 0xff).ok()?);
        }
        _ => return None,
    }
    Some(output)
}
