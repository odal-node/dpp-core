//! What each EN 18219 clause 5 scheme accepts, and what it refuses.

use serde_json::json;

use super::{ProductIdentifier, ProductIdentifierError};
use crate::identifier::Gtin;

const GTIN: &str = "03801234567898";

#[test]
fn a_scheme_1_identifier_carries_a_validated_gtin() {
    let id = ProductIdentifier::gs1(Gtin::parse(GTIN).unwrap());
    assert_eq!(id.gtin().map(Gtin::as_str), Some(GTIN));
    assert_eq!(id.as_str(), GTIN);
}

/// The whole point of the type: schemes 2 and 3 have no GTIN, and a caller must
/// be made to notice rather than receive an empty string.
#[test]
fn the_self_issuing_schemes_have_no_gtin() {
    let link = ProductIdentifier::identification_link("https://id.example.com/p/1").unwrap();
    let did = ProductIdentifier::did("did:web:example.com:p:1").unwrap();

    assert!(link.gtin().is_none());
    assert!(did.gtin().is_none());
    // …but they still identify something, which is what `as_str` is for.
    assert_eq!(link.as_str(), "https://id.example.com/p/1");
    assert_eq!(did.as_str(), "did:web:example.com:p:1");
}

#[test]
fn an_identification_link_must_be_an_absolute_web_url() {
    for bad in [
        "example.com/p/1",
        "/p/1",
        "ftp://example.com/p/1",
        "https://",
        "",
    ] {
        assert!(
            matches!(
                ProductIdentifier::identification_link(bad),
                Err(ProductIdentifierError::NotAWebUrl(_))
            ),
            "{bad:?} was accepted as an identification link"
        );
    }
    assert!(ProductIdentifier::identification_link("http://example.com/p/1").is_ok());
}

/// Clause 5 names `web`, `ethr` and `ebsi`. A method outside that set is refused
/// rather than carried: an identifier nobody can resolve points at no passport,
/// and accepting it would let one be created that is unreachable by design.
#[test]
fn only_the_did_methods_the_standard_names_are_accepted() {
    for method in ["web", "ethr", "ebsi"] {
        let did = format!("did:{method}:example.com:p:1");
        assert!(
            ProductIdentifier::did(&did).is_ok(),
            "{did} was refused but clause 5 names it"
        );
    }
    assert!(matches!(
        ProductIdentifier::did("did:key:z6Mk"),
        Err(ProductIdentifierError::UnsupportedDidMethod(m)) if m == "key"
    ));
}

#[test]
fn a_malformed_did_is_refused() {
    assert!(matches!(
        ProductIdentifier::did("web:example.com"),
        Err(ProductIdentifierError::NotADid(_))
    ));
    assert!(matches!(
        ProductIdentifier::did("did:web"),
        Err(ProductIdentifierError::NotADid(_))
    ));
    assert!(matches!(
        ProductIdentifier::did("did:web:"),
        Err(ProductIdentifierError::EmptyDidMethodId(_))
    ));
}

/// The wire form is the thing a stored passport will carry, so it is pinned as a
/// literal rather than round-tripped — a round-trip agrees with whatever the
/// type currently emits and cannot notice a rename.
#[test]
fn each_scheme_serialises_to_its_documented_wire_form() {
    let cases = [
        (
            ProductIdentifier::gs1(Gtin::parse(GTIN).unwrap()),
            json!({"scheme": "gs1", "gtin": GTIN}),
        ),
        (
            ProductIdentifier::identification_link("https://id.example.com/p/1").unwrap(),
            json!({"scheme": "identificationLink", "url": "https://id.example.com/p/1"}),
        ),
        (
            ProductIdentifier::did("did:web:example.com:p:1").unwrap(),
            json!({"scheme": "did", "did": "did:web:example.com:p:1"}),
        ),
    ];

    for (value, wire) in cases {
        assert_eq!(serde_json::to_value(&value).unwrap(), wire);
        let back: ProductIdentifier = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(back, value);
    }
}

/// Deserialisation must not be a way around the constructors' validation — a
/// stored document is as much an input as a constructor argument.
#[test]
fn deserialisation_still_rejects_an_invalid_gtin() {
    let bad = json!({"scheme": "gs1", "gtin": "03801234567891"});
    assert!(
        serde_json::from_value::<ProductIdentifier>(bad).is_err(),
        "a bad check digit was accepted through serde"
    );
}

/// 🚨 A gap, asserted so it is not mistaken for coverage: `serde` builds the
/// scheme 2 and 3 arms field-by-field and never calls the constructors, so a
/// stored `identificationLink` or `did` is **not** revalidated on read.
///
/// `Gtin` does not have this problem — it validates in its own `Deserialize`,
/// which is why the test above passes. The two self-issuing arms hold plain
/// `String`s, so there is nowhere for that check to live short of a custom
/// `Deserialize` on each.
///
/// Left as it is for now because nothing persists this type yet. It has to be
/// closed by the change that does — a document is an input, and an identifier
/// that could not have been constructed must not be readable either.
#[test]
fn a_stored_identifier_is_not_revalidated_and_this_is_the_known_gap() {
    let unconstructable = json!({"scheme": "did", "did": "did:key:z6Mk"});
    let read: ProductIdentifier = serde_json::from_value(unconstructable).unwrap();
    assert_eq!(read.as_str(), "did:key:z6Mk");
    assert!(
        ProductIdentifier::did("did:key:z6Mk").is_err(),
        "the constructor refuses what serde just accepted — if this line fails, \
         the gap is closed and this test should be deleted"
    );
}
