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
        // 🚨 The near-misses. Each leaves a non-empty remainder after the
        // scheme and carries no host at all, so a check for "something follows
        // `https://`" let all four through.
        "https:///p/1",
        "https://?product=1",
        "https://#fragment",
        "https://exam ple.com/p/1",
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

/// 🚨 An identification link is printable ASCII, and it is refused rather than
/// converted.
///
/// EN 18219:2026 clause 4.3 makes a unique identifier an ISO/IEC 646 string, and
/// as a URI it is percent-encoded. Both of these were accepted: each has a host
/// and no whitespace. A link stored raw and the same link stored percent-encoded
/// would then be two identifiers for one product.
#[test]
fn an_identification_link_outside_printable_ascii_is_refused_with_its_remedy() {
    // A raw non-ASCII host: punycode is the remedy, and the error says so.
    let host = ProductIdentifier::identification_link("https://exämple.com/item")
        .expect_err("a Unicode host is not ASCII");
    assert_eq!(
        host,
        ProductIdentifierError::NonAsciiHost {
            url: "https://exämple.com/item".into(),
            character: 'ä',
        }
    );
    assert!(host.to_string().contains("punycode"), "{host}");

    // A raw non-ASCII path: percent-encoding is the remedy.
    let path = ProductIdentifier::identification_link("https://example.com/ü")
        .expect_err("a raw non-ASCII path is not ASCII");
    assert_eq!(
        path,
        ProductIdentifierError::NotPrintableAscii {
            url: "https://example.com/ü".into(),
            character: 'ü',
        }
    );
    assert!(path.to_string().contains("percent-encode"), "{path}");

    // Written as the standard asks, both are accepted, and stored as given.
    for url in [
        "https://example.com/%C3%BC",
        "https://xn--exmple-cua.com/item",
    ] {
        let link = ProductIdentifier::identification_link(url).expect("printable ASCII");
        assert_eq!(
            link.as_str(),
            url,
            "the stored identifier is the printed one"
        );
    }
}

/// 🚨 Reading a document is not issuing an identifier: a link outside printable
/// ASCII is refused by the constructor and **still read** from a document, as it
/// is, never converted.
///
/// The product identifier is in the signed public view, so a passport carrying one
/// can arrive from another operator, signed and unrewritable. A reader that
/// refused it would refuse correct data permanently, and report a node that has not
/// upgraded as having tampered with it. The fetched side decides, because it is
/// the stricter. A stored document is not turned away either; the plugin refuses
/// the link the next time the passport is written, naming the field and the
/// remedy.
#[test]
fn a_document_carrying_a_non_ascii_link_is_still_read_as_it_is() {
    for url in [
        "https://exämple.com/item",
        "https://example.com/ü",
        "https://example.com/\u{7f}",
    ] {
        assert!(
            ProductIdentifier::identification_link(url).is_err(),
            "the constructor accepted {url}"
        );
        let read: ProductIdentifier =
            serde_json::from_value(json!({"scheme": "identificationLink", "url": url}))
                .unwrap_or_else(|e| panic!("{url} no longer reads: {e}"));
        assert_eq!(read.as_str(), url, "reading must not convert the link");
        assert_eq!(
            serde_json::to_value(&read).expect("serialises"),
            json!({"scheme": "identificationLink", "url": url}),
            "and writing it back must not either: a signed document stays byte-for-byte"
        );
    }
}

/// What reading has always refused, it still refuses: a value that is not a web
/// URL at all has nothing for a reader to resolve.
#[test]
fn a_document_carrying_a_link_that_is_not_a_web_url_is_still_refused() {
    for url in [
        "https:///ü",
        "ftp://example.com/p/1",
        "https://exa mple.com/",
    ] {
        assert!(
            serde_json::from_value::<ProductIdentifier>(
                json!({"scheme": "identificationLink", "url": url})
            )
            .is_err(),
            "{url} read"
        );
    }
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

/// W3C DID v1.0 clause 3.1: `method-specific-id = *( *idchar ":" ) 1*idchar`,
/// `idchar = ALPHA / DIGIT / "." / "-" / "_" / pct-encoded`.
///
/// Checking the prefix, the method and "something follows it" accepted values
/// the grammar excludes — `did:web: ` being the plainest, a DID whose identifier
/// is one space. Nothing resolves it, and it would have been stored as though it
/// identified a product.
#[test]
fn a_did_must_satisfy_the_did_core_grammar() {
    for bad in [
        "did:web: ",
        "did:web:exam ple.com",
        "did:web:example.com:",
        "did:web:exa%mple",
        "did:web:example%",
        "did:web:example%zz",
        "did:web:example.com/p/1",
        "did:web:example.com?q=1",
    ] {
        assert!(
            matches!(
                ProductIdentifier::did(bad),
                Err(ProductIdentifierError::NotADid(_))
            ),
            "{bad:?} was accepted as a DID"
        );
    }

    // …and what the grammar does admit, including a percent-encoded port and an
    // empty intermediate segment, which `*( *idchar ":" )` permits.
    for good in [
        "did:web:example.com%3A3000:p:1",
        "did:web:example.com::p",
        "did:web:a_b-c.d",
    ] {
        assert!(
            ProductIdentifier::did(good).is_ok(),
            "{good:?} was refused but the grammar admits it"
        );
    }
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

/// The self-issuing arms hold plain `String`s, so deriving `Deserialize` built
/// them field-by-field and never called the constructors — a stored DID or
/// identification link that the constructor refuses read back happily. `Gtin`
/// never had the problem, because it validates in its own `Deserialize`, and
/// that is what made the hole easy to miss: scheme 1 was safe and the other two
/// were not.
///
/// Every value below is one the constructor refuses. Read and construct must
/// agree, because a document is as much an input as a constructor argument.
#[test]
fn deserialisation_refuses_what_the_constructors_refuse() {
    let unconstructable = [
        json!({"scheme": "did", "did": "did:key:z6Mk"}),
        json!({"scheme": "did", "did": "did:web: "}),
        json!({"scheme": "did", "did": "did:web:"}),
        json!({"scheme": "identificationLink", "url": "https:///p/1"}),
        json!({"scheme": "identificationLink", "url": "ftp://example.com/p/1"}),
        json!({"scheme": "identificationLink", "url": "example.com/p/1"}),
    ];

    for wire in unconstructable {
        assert!(
            serde_json::from_value::<ProductIdentifier>(wire.clone()).is_err(),
            "{wire} was accepted through serde but the constructor refuses it"
        );
    }
}
