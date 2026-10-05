//! RFC 9562 appendix A.6's UUIDv7 example, run through the layout
//! [`PassportId::default_carrier_serial`] relies on.
//!
//! The serial is the ten bytes after the six-byte millisecond timestamp, so that
//! nothing printed on a carrier reveals when its passport was made. That holds
//! only if a UUIDv7 has the layout the RFC defines. `id_tests.rs` builds its ids
//! from bytes chosen by the code's author, so a misreading of the layout would
//! pass them. This uses the RFC's own example, whose timestamp, version, variant
//! and random fields the RFC states.

use uuid::Uuid;

use super::id::PassportId;

/// Appendix A.6's final value.
const RFC_EXAMPLE: &str = "017F22E2-79B0-7CC3-98C4-DC0C0C07398F";

/// The same appendix's timestamp: 22 February 2022, 2:22:22 pm at UTC-5, in
/// milliseconds since the Unix epoch.
const RFC_TIMESTAMP_MS: u64 = 1_645_557_742_000;

fn example() -> Uuid {
    Uuid::parse_str(RFC_EXAMPLE).expect("the RFC's example parses")
}

/// The fields the RFC lists for the example, read back from the parsed value:
/// version 7, the variant bits `0b10`, and the timestamp in the leading six
/// bytes.
#[test]
fn the_rfc_example_has_the_fields_the_rfc_lists() {
    let id = example();

    assert_eq!(id.get_version_num(), 7);
    assert_eq!(id.as_bytes()[8] >> 6, 0b10, "the two variant bits");
    assert_eq!(
        id.get_timestamp().map(|t| t.to_unix()),
        Some((RFC_TIMESTAMP_MS / 1000, 0))
    );

    let [a, b, c, d, e, f, ..] = *id.as_bytes();
    assert_eq!(
        u64::from_be_bytes([0, 0, a, b, c, d, e, f]),
        RFC_TIMESTAMP_MS,
        "the leading six bytes are a big-endian millisecond timestamp"
    );
}

/// The derivation takes everything after the timestamp, and none of it.
#[test]
fn the_carrier_serial_is_the_bytes_after_the_rfc_timestamp() {
    let serial = PassportId(example()).default_carrier_serial();

    assert_eq!(serial, "7cc398c4dc0c0c07398f");
    assert_eq!(serial.len(), 20);
    assert!(
        !serial.contains("017f22e279b0"),
        "the timestamp must not appear in a code printed on a product"
    );
}
