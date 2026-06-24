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

/// A decoded Bitstring Status List — the GZIP-decompressed bitstring.
#[derive(Debug, Clone)]
pub struct StatusList {
    bits: Vec<u8>,
}

impl StatusList {
    /// Decode a W3C `encodedList`: an optional multibase `u` prefix followed by
    /// base64url (no padding) of a GZIP-compressed bitstring.
    pub fn from_encoded_list(encoded: &str) -> anyhow::Result<Self> {
        let b64 = encoded.strip_prefix('u').unwrap_or(encoded);
        let compressed = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(b64)
            .map_err(|e| anyhow::anyhow!("status list base64url: {e}"))?;
        let mut bits = Vec::new();
        GzDecoder::new(&compressed[..])
            .read_to_end(&mut bits)
            .map_err(|e| anyhow::anyhow!("status list gunzip: {e}"))?;
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
