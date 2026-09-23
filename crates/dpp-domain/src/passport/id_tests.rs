//! The default carrier serial: its shape, its equivalence with the id's text
//! form, and the two creation-time leaks it must not reintroduce.

use uuid::Uuid;

use super::id::PassportId;

fn id(bytes: [u8; 16]) -> PassportId {
    PassportId(Uuid::from_bytes(bytes))
}

#[test]
fn the_default_is_twenty_hex_characters_from_the_random_tail() {
    let serial = id([
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc,
        0xfe,
    ])
    .default_carrier_serial();
    assert_eq!(serial, "cdef1032547698badcfe");
    assert_eq!(
        dpp_rules::common::identifier::check_gs1_serial(&serial),
        Ok(())
    );
}

/// Storage indexes the default as an expression over the id's text form, so
/// the two derivations have to be the same string.
#[test]
fn the_default_is_the_tail_of_the_canonical_id() {
    let passport = PassportId::new();
    let canonical = passport.to_string().replace('-', "");
    assert_eq!(passport.default_carrier_serial(), canonical[12..]);
}

/// Regression: the serial must not carry the passport's creation time.
#[test]
fn the_default_does_not_embed_the_uuid_timestamp() {
    let mut bytes = [0u8; 16];
    bytes[..6].copy_from_slice(&[0x01, 0x9f, 0x99, 0xa0, 0xde, 0xad]);
    bytes[6] = 0x7a;
    bytes[7..].copy_from_slice(&[0xbc, 0x8d, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab]);

    let serial = id(bytes).default_carrier_serial();
    assert!(
        !serial.contains("019f99a0dead"),
        "the default must not carry the UUIDv7 timestamp: {serial}"
    );
}

/// Regression: defaults must not sort in creation order, or two batteries
/// produced minutes apart would reveal their production order.
#[test]
fn defaults_do_not_order_by_creation_time() {
    let mut earlier = [0u8; 16];
    earlier[..6].copy_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x01]);
    earlier[6..].copy_from_slice(&[0x7f, 0xff, 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);

    let mut later = [0u8; 16];
    later[..6].copy_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x02]);
    later[6..].copy_from_slice(&[0x70, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    assert!(id(later).default_carrier_serial() < id(earlier).default_carrier_serial());
}
