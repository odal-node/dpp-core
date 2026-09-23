//! RFC 9901's own examples, checked against this implementation.
//!
//! Every other test in this module issues with this code and verifies with
//! this code, so a shared misreading — hashing the decoded bytes instead of the
//! encoded string, or the wrong base64 alphabet — would pass all of them and
//! produce credentials no other implementation accepts. These compare against
//! values the RFC publishes, which this code did not produce.

use serde_json::json;

use super::{Disclosure, digest_of};

/// RFC 9901 clause 4.2.1: the Disclosure for `["_26bc4LT-ac6q2KI6cBW5es",
/// "family_name", "Möbius"]`, as the RFC prints it.
const RFC_DISCLOSURE: &str =
    "WyJfMjZiYzRMVC1hYzZxMktJNmNCVzVlcyIsICJmYW1pbHlfbmFtZSIsICJNw7ZiaXVzIl0";

/// RFC 9901 clause 4.2.3: that Disclosure's base64url SHA-256 digest.
const RFC_DIGEST: &str = "X9yH0Ajrdm1Oij4tWso9UzzKJvPoDxwmuEcO3XAdRC0";

/// Clause 4.2.1's three further encodings of the same claim, each valid:
/// the umlaut escaped, no whitespace, and newlines between elements.
const RFC_VARIANTS: [&str; 3] = [
    "WyJfMjZiYzRMVC1hYzZxMktJNmNCVzVlcyIsICJmYW1pbHlfbmFtZSIsICJNXHUwMGY2Yml1cyJd",
    "WyJfMjZiYzRMVC1hYzZxMktJNmNCVzVlcyIsImZhbWlseV9uYW1lIiwiTcO2Yml1cyJd",
    "WwoiXzI2YmM0TFQtYWM2cTJLSTZjQlc1ZXMiLAoiZmFtaWx5X25hbWUiLAoiTcO2Yml1cyIKXQ",
];

#[test]
fn the_rfc_example_disclosure_hashes_to_its_published_digest() {
    let d = Disclosure::parse(RFC_DISCLOSURE).expect("the RFC's disclosure parses");
    assert_eq!(d.salt(), "_26bc4LT-ac6q2KI6cBW5es");
    assert_eq!(d.claim_name(), "family_name");
    assert_eq!(d.claim_value(), &json!("Möbius"));
    assert_eq!(d.digest(), RFC_DIGEST);
    assert_eq!(digest_of(RFC_DISCLOSURE), RFC_DIGEST);
}

/// The RFC says why the digest is over the string: every encoding of one claim
/// reads the same, and each hashes differently. A digest recomputed from the
/// parsed triple would give all four the same value and break every one of
/// them but the one this code happens to produce.
#[test]
fn every_rfc_encoding_of_one_claim_reads_the_same_and_hashes_apart() {
    let mut digests = vec![digest_of(RFC_DISCLOSURE)];
    for variant in RFC_VARIANTS {
        let d = Disclosure::parse(variant).expect("each RFC variant parses");
        assert_eq!(d.claim_name(), "family_name", "{variant}");
        assert_eq!(d.claim_value(), &json!("Möbius"), "{variant}");
        assert_eq!(d.digest(), digest_of(variant), "{variant}");
        digests.push(d.digest());
    }
    digests.sort();
    digests.dedup();
    assert_eq!(digests.len(), 4, "four encodings, four digests");
}

/// Encoding, not only reading: the triple this code serialises is the RFC's
/// no-whitespace form byte for byte, the umlaut as raw UTF-8.
#[test]
fn this_encoder_produces_the_rfc_compact_form() {
    let d = Disclosure::with_salt(
        "_26bc4LT-ac6q2KI6cBW5es".to_owned(),
        "family_name",
        json!("Möbius"),
    )
    .expect("a permitted claim name");
    assert_eq!(d.encoded(), RFC_VARIANTS[1]);
}
