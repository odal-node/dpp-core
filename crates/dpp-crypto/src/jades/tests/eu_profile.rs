//! The profile the EU recognises.

use serde_json::Value;

use super::super::*;
use super::common::*;

#[test]
fn only_the_chain_with_thumbprint_form_is_in_the_eu_recognised_profile() {
    // Commission Implementing Regulation (EU) 2026/248 Annex I makes `x5c`
    // mandatory; TS 119 182-1 Table 1 makes a digest reference mandatory. Each
    // of the other two variants satisfies exactly one.
    assert!(
        !CertificateRef::thumbprint_of_der(FAKE_DER).is_eu_recognised_profile(),
        "x5t#S256 alone carries no x5c, so Annex I does not reach it"
    );
    assert!(
        !CertificateRef::Chain(vec!["Y2VydA==".into()]).is_eu_recognised_profile(),
        "x5c alone fails Table 1's certificate-reference service"
    );
    assert!(
        CertificateRef::chain_of_der(std::slice::from_ref(&FAKE_DER.to_vec()))
            .expect("one certificate")
            .is_eu_recognised_profile()
    );
}

#[test]
fn the_eu_recognised_form_actually_emits_x5c() {
    // Pins the adaptation itself rather than the variant name. The requirement
    // lives in a Regulation, so a reader checking this module against the ETSI
    // document alone would never meet it — this is where it is written down in a
    // form that fails if the emitted header changes.
    let header = JadesHeader::now(
        "EdDSA",
        CertificateRef::chain_of_der(std::slice::from_ref(&FAKE_DER.to_vec()))
            .expect("one certificate"),
    );
    let bytes = header.to_json_bytes().expect("header builds");
    let protected: serde_json::Map<String, Value> =
        serde_json::from_slice(&bytes).expect("the protected header is a JSON object");

    assert!(
        header.certificate.is_eu_recognised_profile(),
        "the form under test must be the one Annex I lists"
    );
    assert!(
        protected.contains_key("x5c"),
        "Annex I: the x5c header parameter shall be present — got {:?}",
        protected.keys().collect::<Vec<_>>()
    );
    assert!(
        protected.contains_key("x5t#S256"),
        "TS 119 182-1 Table 1 still wants a digest reference alongside it"
    );
}

/// 🚨 `is_eu_recognised_profile` answered on the variant alone, so a
/// `ChainWithThumbprint` carrying nothing was reported as the EU-recognised
/// format.
///
/// `chain_of_der` cannot build one — but the variant's fields are public, so
/// this value is one any caller can make, and a compliance caller acting on the
/// answer would have accepted a header identifying no certificate at all. Both
/// legs the method documents are *presence* requirements: Annex I's `x5c`
/// "shall be present", and Table 1's digest reference with cardinality 1. An
/// empty one is present and references nothing.
#[test]
fn an_empty_chain_or_thumbprint_is_not_the_eu_recognised_profile() {
    let cases = [
        (
            "empty chain",
            CertificateRef::ChainWithThumbprint {
                chain: vec![],
                thumbprint: "abc".into(),
            },
        ),
        (
            "empty thumbprint",
            CertificateRef::ChainWithThumbprint {
                chain: vec!["Y2VydA==".into()],
                thumbprint: String::new(),
            },
        ),
        (
            "both empty",
            CertificateRef::ChainWithThumbprint {
                chain: vec![],
                thumbprint: String::new(),
            },
        ),
    ];

    for (name, reference) in cases {
        assert!(
            !reference.is_eu_recognised_profile(),
            "{name} was reported as the format a Member State body must recognise"
        );
    }

    // And the populated form still is, so the fix is not "always false".
    assert!(
        CertificateRef::ChainWithThumbprint {
            chain: vec!["Y2VydA==".into()],
            thumbprint: "abc".into(),
        }
        .is_eu_recognised_profile()
    );
}

/// The serialiser has to refuse the same two, or it writes a header the
/// predicate has just said is not conformant. It caught the empty chain and not
/// the empty thumbprint.
#[test]
fn a_header_with_an_empty_thumbprint_is_refused_like_an_empty_chain() {
    let h = JadesHeader {
        certificate: CertificateRef::ChainWithThumbprint {
            chain: vec!["Y2VydA==".into()],
            thumbprint: String::new(),
        },
        ..header()
    };
    assert_eq!(
        prepare(&h, b"payload").expect_err("an empty x5t#S256 references no certificate"),
        JadesError::EmptyThumbprint
    );
}

/// A thumbprint that is not the digest of `chain[0]` still answers the
/// predicate, which checks presence only. Serialising the header is where the
/// two halves are compared, and it refuses the pair, so a header whose digest
/// names another certificate is never written.
#[test]
fn a_thumbprint_of_another_certificate_is_refused_on_serialise() {
    let CertificateRef::Thumbprint(other) =
        CertificateRef::thumbprint_of_der(b"another certificate")
    else {
        unreachable!("thumbprint_of_der always yields a Thumbprint");
    };
    let mismatched = CertificateRef::ChainWithThumbprint {
        chain: vec!["Y2VydA==".into()],
        thumbprint: other,
    };
    assert!(
        mismatched.is_eu_recognised_profile(),
        "the predicate checks presence, so a digest of another certificate still answers true"
    );
    let h = JadesHeader {
        certificate: mismatched,
        ..header()
    };
    assert_eq!(
        h.to_json_bytes()
            .expect_err("a digest of another certificate is refused"),
        JadesError::ThumbprintMismatch
    );
}
