//! The list of section 4: the draft's own examples, then what a reader refuses.

use std::io::Write;

use base64::Engine;
use flate2::Compression;
use flate2::write::GzEncoder;

use super::super::{MAX_DECODED_BYTES, TokenStatusList, TokenStatusListError};

/// Section 4.1, one bit per status: sixteen entries in two bytes. The bytes, the
/// compressed hex and the `lst` of section 4.2 are the draft's, and the three agree
/// with each other, which `the_drafts_strings_are_the_same_bytes` checks, so a
/// transcription slip in one shows against the others.
const ONE_BIT_BYTES: [u8; 2] = [0xB9, 0xA3];
const ONE_BIT_ZLIB_HEX: &str = "78dadbb918000217015d";
const ONE_BIT_LST: &str = "eNrbuRgAAhcBXQ";
/// What those bytes hold, entry by entry, counting bits from the least significant
/// (section 4.1 step 3).
const ONE_BIT_STATUSES: [u8; 16] = [1, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1];

/// Section 4.1, two bits per status: twelve entries in three bytes.
const TWO_BIT_BYTES: [u8; 3] = [0xC9, 0x44, 0xF9];
const TWO_BIT_ZLIB_HEX: &str = "78da3be9f2130003df0207";
const TWO_BIT_LST: &str = "eNo76fITAAPfAgc";
const TWO_BIT_STATUSES: [u8; 12] = [1, 2, 0, 3, 0, 1, 0, 1, 1, 2, 3, 3];

fn unpadded(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[test]
fn the_drafts_strings_are_the_same_bytes() {
    for (hex, lst) in [
        (ONE_BIT_ZLIB_HEX, ONE_BIT_LST),
        (TWO_BIT_ZLIB_HEX, TWO_BIT_LST),
    ] {
        assert_eq!(unpadded(&hex::decode(hex).unwrap()), lst);
    }
}

#[test]
fn the_drafts_one_bit_list_reads_as_the_draft_lays_it_out() {
    let list = TokenStatusList::from_encoded(1, ONE_BIT_LST).unwrap();
    assert_eq!(
        list,
        TokenStatusList::from_bytes(1, ONE_BIT_BYTES.to_vec()).unwrap()
    );
    assert_eq!(list.entries(), 16);
    let read: Vec<u8> = (0..16).map(|idx| list.get(idx).unwrap()).collect();
    assert_eq!(read, ONE_BIT_STATUSES);
}

#[test]
fn the_drafts_two_bit_list_reads_as_the_draft_lays_it_out() {
    let list = TokenStatusList::from_encoded(2, TWO_BIT_LST).unwrap();
    assert_eq!(
        list,
        TokenStatusList::from_bytes(2, TWO_BIT_BYTES.to_vec()).unwrap()
    );
    assert_eq!(list.entries(), 12);
    let read: Vec<u8> = (0..12).map(|idx| list.get(idx).unwrap()).collect();
    assert_eq!(read, TWO_BIT_STATUSES);
}

/// Not required by the draft, which asks only for DEFLATE in the zlib format. If
/// the encoder here writes the draft's own bytes, a list it makes is the list the
/// draft shows, and a difference in a later `flate2` shows up here.
#[test]
fn encoding_the_drafts_lists_writes_the_drafts_strings() {
    let one = TokenStatusList::from_bytes(1, ONE_BIT_BYTES.to_vec()).unwrap();
    let two = TokenStatusList::from_bytes(2, TWO_BIT_BYTES.to_vec()).unwrap();
    assert_eq!(one.encode().unwrap(), ONE_BIT_LST);
    assert_eq!(two.encode().unwrap(), TWO_BIT_LST);
}

#[test]
fn every_width_keeps_what_was_set() {
    for bits in [1u8, 2, 4, 8] {
        // Every entry takes its own value, cycling through all that fit, so that a
        // neighbour bleeding into another shows.
        let distinct = u64::from(1u16 << bits);
        let values: Vec<u8> = (0..40u64)
            .map(|idx| u8::try_from(idx % distinct).unwrap())
            .collect();
        let mut list = TokenStatusList::new(bits, 40).unwrap();
        for (idx, value) in (0u64..).zip(&values) {
            list.set(idx, *value).unwrap();
        }
        let again =
            TokenStatusList::from_encoded(bits, &list.encode().expect("compresses")).unwrap();
        for (idx, value) in (0u64..).zip(&values) {
            assert_eq!(again.get(idx), Some(*value), "bits {bits}, idx {idx}");
        }
    }
}

#[test]
fn setting_an_entry_leaves_its_neighbours() {
    let mut list = TokenStatusList::from_bytes(2, vec![0b1111_1111]).unwrap();
    list.set(1, 0).unwrap();
    assert_eq!([0, 1, 2, 3].map(|idx| list.get(idx).unwrap()), [3, 0, 3, 3]);
}

/// The two Bitstring formats count from opposite ends of a byte, so a reader of
/// one cannot read the other, and a list decoded with the wrong one would flip
/// the first entry of every byte.
#[test]
fn the_first_entry_is_the_least_significant_bit_unlike_the_w3c_list() {
    let token = TokenStatusList::from_bytes(1, vec![0b0000_0001]).unwrap();
    let w3c = crate::status_list::StatusList::from_bitstring(vec![0b0000_0001]);
    assert_eq!(token.get(0), Some(1));
    assert_eq!(w3c.get(0), Some(false));
}

#[test]
fn an_index_past_the_end_reads_nothing() {
    let list = TokenStatusList::from_bytes(4, vec![0x21, 0x43]).unwrap();
    assert_eq!(list.entries(), 4);
    assert_eq!(list.get(3), Some(4));
    assert_eq!(list.get(4), None);
    assert_eq!(list.get(u64::MAX), None);
}

#[test]
fn a_width_the_draft_does_not_allow_is_refused() {
    for bits in [0u8, 3, 5, 6, 7, 9, 16, 255] {
        let refused = Err(TokenStatusListError::UnsupportedBits(u64::from(bits)));
        assert_eq!(TokenStatusList::new(bits, 8), refused);
        assert_eq!(TokenStatusList::from_bytes(bits, vec![0]), refused);
        assert_eq!(TokenStatusList::from_encoded(bits, ONE_BIT_LST), refused);
    }
}

#[test]
fn setting_refuses_what_does_not_fit_or_is_not_there() {
    let mut list = TokenStatusList::new(2, 8).unwrap();
    assert_eq!(
        list.set(0, 4),
        Err(TokenStatusListError::ValueTooWide { value: 4, bits: 2 })
    );
    assert_eq!(
        list.set(8, 1),
        Err(TokenStatusListError::IndexOutOfBounds { idx: 8, entries: 8 })
    );
    assert_eq!(list, TokenStatusList::new(2, 8).unwrap(), "nothing changed");
}

#[test]
fn a_new_list_is_all_valid_and_rounded_up_to_a_byte() {
    let list = TokenStatusList::new(2, 5).unwrap();
    assert_eq!(list.entries(), 8);
    assert!((0..8).all(|idx| list.get(idx) == Some(0)));
}

#[test]
fn a_list_too_large_to_hold_is_refused() {
    let too_large = Err(TokenStatusListError::TooLarge(MAX_DECODED_BYTES));
    assert_eq!(
        TokenStatusList::new(8, MAX_DECODED_BYTES as u64 + 1),
        too_large
    );
    assert_eq!(TokenStatusList::new(8, u64::MAX), too_large);
}

#[test]
fn a_list_that_expands_past_the_budget_is_refused() {
    let lst = TokenStatusList::from_bytes(8, vec![0; 4096])
        .unwrap()
        .encode()
        .unwrap();
    assert!(matches!(
        TokenStatusList::from_encoded_within(8, &lst, 1024),
        Err(TokenStatusListError::TooLarge(1024))
    ));
    assert!(TokenStatusList::from_encoded_within(8, &lst, 4096).is_ok());
    assert!(TokenStatusList::from_encoded_within(8, &lst, usize::MAX).is_ok());
}

/// A few kilobytes of input that ask for more memory than the default budget: the
/// shape the budget is for.
#[test]
fn a_small_input_that_expands_past_the_default_budget_is_refused() {
    let lst = TokenStatusList::from_bytes(8, vec![0; MAX_DECODED_BYTES + 1])
        .unwrap()
        .encode()
        .unwrap();
    assert!(lst.len() < 64 * 1024, "the fixture should compress well");
    assert_eq!(
        TokenStatusList::from_encoded(8, &lst),
        Err(TokenStatusListError::TooLarge(MAX_DECODED_BYTES))
    );
}

#[test]
fn text_that_is_not_unpadded_base64url_is_refused() {
    for lst in [
        // Padded, which RFC 7515 base64url is not.
        "eNrbuRgAAhcBXQ==",
        // The standard alphabet.
        "eNrb+RgAAhcBXQ",
        "eNrb/RgAAhcBXQ",
        "not base64!",
    ] {
        assert_eq!(
            TokenStatusList::from_encoded(1, lst),
            Err(TokenStatusListError::NotBase64Url),
            "{lst}"
        );
    }
}

#[test]
fn bytes_that_are_not_a_zlib_stream_are_refused() {
    let not_zlib = TokenStatusList::from_encoded(1, &unpadded(b"this is not zlib"));
    assert!(matches!(not_zlib, Err(TokenStatusListError::NotZlib(_))));
}

/// The W3C list is GZIP. Reading it as a Token Status List would be reading
/// another format's bytes as this one's, so it is refused.
#[test]
fn a_gzip_stream_is_not_a_status_list() {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::best());
    gzip.write_all(&ONE_BIT_BYTES).unwrap();
    let lst = unpadded(&gzip.finish().unwrap());
    assert!(matches!(
        TokenStatusList::from_encoded(1, &lst),
        Err(TokenStatusListError::NotZlib(_))
    ));
}
