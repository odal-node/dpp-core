//! The Status List of draft-ietf-oauth-status-list-21 section 4: an array of
//! small status values, packed into bytes and compressed.

use std::io::{Read, Write};

use base64::Engine;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use super::error::TokenStatusListError;

/// Largest list [`TokenStatusList::from_encoded`] accepts once decompressed.
///
/// The same budget as the W3C list's ([`crate::status_list::MAX_DECODED_BYTES`]),
/// for the same reason: the draft sets no limit, and the compressed input is
/// small enough to ask for far more memory than it is worth.
pub const MAX_DECODED_BYTES: usize = crate::status_list::MAX_DECODED_BYTES;

/// A list of status values, each `bits` wide.
///
/// Entry `idx` sits in the `bits` bits starting at bit `idx * bits`, counted from
/// the *least* significant bit of the first byte (section 4.1 step 3). That is the
/// reverse of the W3C Bitstring Status List, which counts from the most
/// significant: the two cannot share a reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenStatusList {
    bits: u8,
    bytes: Vec<u8>,
}

impl TokenStatusList {
    /// A list of `entries` entries, all `0x00` (valid), as a status issuer starts.
    ///
    /// The array is rounded up to whole bytes, so the list can hold more entries
    /// than asked for.
    ///
    /// # Errors
    ///
    /// [`TokenStatusListError::UnsupportedBits`] for a `bits` other than 1, 2, 4 or
    /// 8, and [`TokenStatusListError::TooLarge`] when the array would pass
    /// [`MAX_DECODED_BYTES`].
    pub fn new(bits: u8, entries: u64) -> Result<Self, TokenStatusListError> {
        check_bits(bits)?;
        let too_large = || TokenStatusListError::TooLarge(MAX_DECODED_BYTES);
        let size = entries
            .checked_mul(u64::from(bits))
            .and_then(|bits| bits.checked_add(7))
            .map(|bits| bits / 8)
            .and_then(|bytes| usize::try_from(bytes).ok())
            .filter(|bytes| *bytes <= MAX_DECODED_BYTES)
            .ok_or_else(too_large)?;
        Ok(Self {
            bits,
            bytes: vec![0; size],
        })
    }

    /// A list over bytes already packed as section 4.1 packs them.
    ///
    /// # Errors
    ///
    /// [`TokenStatusListError::UnsupportedBits`] for a `bits` other than 1, 2, 4 or 8.
    pub fn from_bytes(bits: u8, bytes: Vec<u8>) -> Result<Self, TokenStatusListError> {
        check_bits(bits)?;
        Ok(Self { bits, bytes })
    }

    /// Read a list from the `lst` of a Status List Token: unpadded base64url of a
    /// zlib stream, which may not decompress past [`MAX_DECODED_BYTES`].
    ///
    /// # Errors
    ///
    /// See [`TokenStatusListError`].
    pub fn from_encoded(bits: u8, lst: &str) -> Result<Self, TokenStatusListError> {
        Self::from_encoded_within(bits, lst, MAX_DECODED_BYTES)
    }

    /// As [`Self::from_encoded`], refusing a list that decompresses to more than
    /// `max_bytes`. A caller that knows how large its lists are passes a smaller
    /// budget than the default.
    ///
    /// The budget is enforced while decompressing: the reader stops one byte past
    /// it, so the memory taken is bounded by the budget and not by the expansion.
    ///
    /// # Errors
    ///
    /// See [`TokenStatusListError`].
    pub fn from_encoded_within(
        bits: u8,
        lst: &str,
        max_bytes: usize,
    ) -> Result<Self, TokenStatusListError> {
        check_bits(bits)?;
        let compressed = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(lst)
            .map_err(|_| TokenStatusListError::NotBase64Url)?;
        let mut bytes = Vec::new();
        ZlibDecoder::new(&compressed[..])
            .take((max_bytes as u64).saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| TokenStatusListError::NotZlib(e.to_string()))?;
        if bytes.len() > max_bytes {
            return Err(TokenStatusListError::TooLarge(max_bytes));
        }
        Ok(Self { bits, bytes })
    }

    /// The `lst` of a Status List Token: this list compressed at the highest level,
    /// as section 4.1 recommends, and encoded as unpadded base64url.
    ///
    /// # Errors
    ///
    /// [`TokenStatusListError::Compression`] if the compressor fails. Writing into
    /// memory is not expected to, but the compressor's interface can, and a library
    /// path does not assume it cannot.
    pub fn encode(&self) -> Result<String, TokenStatusListError> {
        let compression = |e: std::io::Error| TokenStatusListError::Compression(e.to_string());
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&self.bytes).map_err(compression)?;
        let compressed = encoder.finish().map_err(compression)?;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(compressed))
    }

    /// The width of one entry, 1, 2, 4 or 8.
    #[must_use]
    pub fn bits(&self) -> u8 {
        self.bits
    }

    /// How many entries the list holds.
    #[must_use]
    pub fn entries(&self) -> u64 {
        (self.bytes.len() as u64) * 8 / u64::from(self.bits)
    }

    /// The raw value of entry `idx`, or `None` when it is past the end.
    ///
    /// Read it as a status with [`Status::from_value`](super::Status::from_value).
    #[must_use]
    pub fn get(&self, idx: u64) -> Option<u8> {
        let (byte, shift) = self.position(idx)?;
        Some((self.bytes.get(byte)? >> shift) & self.mask())
    }

    /// Set entry `idx` to `value`.
    ///
    /// # Errors
    ///
    /// [`TokenStatusListError::IndexOutOfBounds`] past the end, and
    /// [`TokenStatusListError::ValueTooWide`] when `value` does not fit in
    /// [`Self::bits`].
    pub fn set(&mut self, idx: u64, value: u8) -> Result<(), TokenStatusListError> {
        if value & !self.mask() != 0 {
            return Err(TokenStatusListError::ValueTooWide {
                value,
                bits: self.bits,
            });
        }
        let entries = self.entries();
        let mask = self.mask();
        let slot = self
            .position(idx)
            .and_then(|(byte, shift)| self.bytes.get_mut(byte).map(|slot| (slot, shift)));
        let Some((slot, shift)) = slot else {
            return Err(TokenStatusListError::IndexOutOfBounds { idx, entries });
        };
        *slot = (*slot & !(mask << shift)) | (value << shift);
        Ok(())
    }

    /// The byte entry `idx` is in and the shift of its bits within that byte.
    /// An entry never straddles two bytes, since `bits` divides 8.
    fn position(&self, idx: u64) -> Option<(usize, u8)> {
        let bit = idx.checked_mul(u64::from(self.bits))?;
        let byte = usize::try_from(bit / 8).ok()?;
        let shift = u8::try_from(bit % 8).ok()?;
        Some((byte, shift))
    }

    fn mask(&self) -> u8 {
        0xFF >> (8 - self.bits)
    }
}

fn check_bits(bits: u8) -> Result<(), TokenStatusListError> {
    match bits {
        1 | 2 | 4 | 8 => Ok(()),
        _ => Err(TokenStatusListError::UnsupportedBits(u64::from(bits))),
    }
}
