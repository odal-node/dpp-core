//! The status list bitstring and its encoding.

use std::io::Read;

use base64::Engine;
use flate2::read::GzDecoder;

/// Largest decompressed bitstring [`StatusList::from_encoded_list`] accepts:
/// 16 MiB, about 134 million entries. The W3C minimum list is 16 KiB, so this
/// leaves room for very large lists while bounding the bitstring one encoded
/// string can make the caller allocate. The base64 decode before it is bounded by
/// the encoded string, which the caller already holds.
pub const MAX_DECODED_BYTES: usize = 16 * 1024 * 1024;

/// A decoded Bitstring Status List — the GZIP-decompressed bitstring.
#[derive(Debug, Clone)]
pub struct StatusList {
    bits: Vec<u8>,
}

impl StatusList {
    /// Decode a W3C `encodedList`: an optional multibase `u` prefix followed by
    /// base64url (no padding) of a GZIP-compressed bitstring.
    ///
    /// The decompressed bitstring may not exceed [`MAX_DECODED_BYTES`]; see
    /// [`Self::from_encoded_list_within`].
    pub fn from_encoded_list(encoded: &str) -> anyhow::Result<Self> {
        Self::from_encoded_list_within(encoded, MAX_DECODED_BYTES)
    }

    /// As [`Self::from_encoded_list`], refusing any bitstring that decompresses
    /// to more than `max_bytes`. A caller that knows how large its list can be
    /// passes a stricter budget than the default.
    ///
    /// GZIP can expand a small input by roughly three orders of magnitude, so the
    /// budget is enforced while decompressing: the reader stops one byte past
    /// `max_bytes`, and the memory allocated is bounded by that budget rather than
    /// by the size of the expansion.
    pub fn from_encoded_list_within(encoded: &str, max_bytes: usize) -> anyhow::Result<Self> {
        let b64 = encoded.strip_prefix('u').unwrap_or(encoded);
        let compressed = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(b64)
            .map_err(|e| anyhow::anyhow!("status list base64url: {e}"))?;
        let mut bits = Vec::new();
        GzDecoder::new(&compressed[..])
            .take((max_bytes as u64).saturating_add(1))
            .read_to_end(&mut bits)
            .map_err(|e| anyhow::anyhow!("status list gunzip: {e}"))?;
        anyhow::ensure!(
            bits.len() <= max_bytes,
            "status list decompresses to more than {max_bytes} bytes"
        );
        Ok(Self { bits })
    }

    /// Build directly from a decompressed bitstring (tests / non-encoded sources).
    pub fn from_bitstring(bits: Vec<u8>) -> Self {
        Self { bits }
    }

    /// Number of status entries this list can address.
    pub fn len_bits(&self) -> usize {
        self.bits.len() * 8
    }

    /// Test the status bit at `index`. Bits are big-endian within each byte: the
    /// zeroth bit is the most-significant bit of the first byte. Returns `None`
    /// when `index` is outside the list (caller should treat as indeterminate).
    pub fn get(&self, index: usize) -> Option<bool> {
        let byte = self.bits.get(index / 8)?;
        Some(byte & (0x80 >> (index % 8)) != 0)
    }
}
