//! W3C Bitstring Status List v1.0 — pure decoding + bit indexing.
//!
//! Revocation/suspension of a Verifiable Credential is expressed by a bit in a
//! shared, published "status list" credential. **Fetching** that credential
//! over the network is an infrastructure concern and lives outside core; this
//! module provides only the pure parts so `dpp-crypto` stays infra-free:
//!
//! 1. decode a status list's `encodedList` (multibase base64url of a GZIP-
//!    compressed bitstring) into the raw bitstring, and
//! 2. test the status bit for a holder's index.
//!
//! Source: <https://www.w3.org/TR/vc-bitstring-status-list/>

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

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;

    fn encode_list(bits: &[u8]) -> String {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(bits).unwrap();
        let gz = enc.finish().unwrap();
        format!(
            "u{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(gz)
        )
    }

    #[test]
    fn a_list_that_expands_past_the_budget_is_refused() {
        let encoded = encode_list(&vec![0u8; 4096]);
        let err = StatusList::from_encoded_list_within(&encoded, 1024).unwrap_err();
        assert!(err.to_string().contains("more than 1024 bytes"), "{err}");
    }

    #[test]
    fn a_list_exactly_at_the_budget_is_accepted() {
        let encoded = encode_list(&vec![0u8; 1024]);
        let list = StatusList::from_encoded_list_within(&encoded, 1024).unwrap();
        assert_eq!(list.len_bits(), 1024 * 8);
    }

    /// The largest budget a caller can pass must not overflow the reader's limit.
    #[test]
    fn the_largest_budget_reads_the_whole_list() {
        let encoded = encode_list(&vec![0u8; 1024]);
        let list = StatusList::from_encoded_list_within(&encoded, usize::MAX).unwrap();
        assert_eq!(list.len_bits(), 1024 * 8);
    }

    #[test]
    fn a_small_input_that_expands_past_the_default_budget_is_refused() {
        // 17 MiB of zeros compresses to a few kilobytes: exactly the shape the
        // default budget exists to refuse.
        let encoded = encode_list(&vec![0u8; MAX_DECODED_BYTES + 1]);
        assert!(encoded.len() < 64 * 1024, "fixture should compress well");
        assert!(StatusList::from_encoded_list(&encoded).is_err());
    }

    #[test]
    fn bit_indexing_is_big_endian_within_byte() {
        let list = StatusList::from_bitstring(vec![0b1000_0001]);
        assert_eq!(list.get(0), Some(true));
        assert_eq!(list.get(1), Some(false));
        assert_eq!(list.get(7), Some(true));
        assert_eq!(list.get(8), None, "out-of-range index is None");
    }

    #[test]
    fn encoded_list_round_trips() {
        let bits = vec![0b0000_0100, 0b0000_0000];
        let encoded = encode_list(&bits);
        let list = StatusList::from_encoded_list(&encoded).expect("decode");
        assert_eq!(list.get(5), Some(true));
        assert_eq!(list.get(4), Some(false));
        assert_eq!(list.get(6), Some(false));
        assert_eq!(list.len_bits(), 16);
    }

    #[test]
    fn decodes_without_multibase_prefix() {
        let bits = vec![0b1000_0000];
        let encoded = encode_list(&bits);
        let without_u = encoded.strip_prefix('u').unwrap();
        let list = StatusList::from_encoded_list(without_u).expect("decode");
        assert_eq!(list.get(0), Some(true));
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(StatusList::from_encoded_list("u!!!not-base64!!!").is_err());
        assert!(StatusList::from_encoded_list("udGhpcyBpcyBub3QgZ3ppcA").is_err());
    }
}
