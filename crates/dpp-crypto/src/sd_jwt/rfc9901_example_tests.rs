//! RFC 9901's end-to-end examples, processed as a Verifier or Holder would.
//!
//! `rfc9901_vector_tests.rs` checks the one thing the RFC pins to the byte: a
//! Disclosure and its digest. This checks what the RFC says follows from them.
//! Each complete SD-JWT in the RFC's Appendix A goes through [`SdJwt::parse`] and
//! [`SdJwt::disclosed_payload`], and what comes out is compared with the
//! Processed SD-JWT Payload the RFC prints. Sections 6.1 to 6.3 are three
//! structures of one address claim, and the RFC prints each Disclosure with its
//! digest.
//!
//! None of that was produced by this code, so a reading of clause 7.1 shared by
//! the code and its tests would show up here: decoy digests mistaken for a
//! mismatch, a recursive Disclosure not opened, a claim left behind that the RFC
//! removes.
//!
//! # What these do not check
//!
//! The signatures. The RFC's examples are signed with ES256, which this crate
//! does not implement, and the module does not verify the Issuer-signed JWT in
//! any case: that is [`crate::jws::verifier`]. Sections 6.1 to 6.3 print only a
//! payload, so those tests give it a stub JWT around it. Key Binding is not
//! checked either: the Key Binding JWT at the end of A.3 and A.4 is tolerated and
//! carried, which is the most this module does with it.

use base64::Engine;
use serde_json::{Map, Value, json};

use super::rfc9901_examples::{
    A1_PRESENTATION, A1_PROCESSED, A3_INPUT, A3_ISSUED, A3_PRESENTATION, A3_PROCESSED,
    A4_PRESENTATION, A4_PROCESSED, S61_ADDRESS, S61_PAYLOAD, S62_DISCLOSURES, S62_PAYLOAD,
    S62_PAYLOAD_COUNTRY_IN_THE_CLEAR, S63_ADDRESS, S63_PAYLOAD,
};
use super::{Disclosure, SdJwt, SdJwtError, digest_of};

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).expect("the RFC's JSON parses")
}

/// What a Verifier is left with after processing `serialised`.
fn processed(serialised: &str) -> Value {
    let sd_jwt = SdJwt::parse(serialised).expect("the RFC's SD-JWT parses");
    Value::Object(
        sd_jwt
            .disclosed_payload()
            .expect("the RFC's SD-JWT processes"),
    )
}

/// A JWT whose payload is the one the RFC prints. The header and the signature
/// are stand-ins: the module under test reads the payload and nothing else.
fn jwt_around(payload: &str) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!(
        "{}.{}.{}",
        b64.encode(br#"{"alg":"ES256"}"#),
        b64.encode(payload),
        b64.encode(b"not a signature")
    )
}

fn disclosure(encoded: &str) -> Disclosure {
    Disclosure::parse(encoded).expect("the RFC's Disclosure parses")
}

/// `payload` read as an SD-JWT carrying `disclosures`, and processed.
fn processed_with(payload: &str, disclosures: &[&str]) -> Result<Value, SdJwtError> {
    let sd_jwt = SdJwt::new(
        jwt_around(payload),
        disclosures.iter().map(|d| disclosure(d)).collect(),
    );
    sd_jwt.disclosed_payload().map(Value::Object)
}

#[test]
fn appendix_a1_presentation_processes_to_the_payload_the_rfc_prints() {
    // Eight digests are in the payload and two Disclosures arrive: the other six
    // are the claims the Holder withheld, and decoys, and all are ignored.
    assert_eq!(processed(A1_PRESENTATION), json_of(A1_PROCESSED));
}

#[test]
fn appendix_a3_presentation_processes_to_the_payload_the_rfc_prints() {
    let sd_jwt = SdJwt::parse(A3_PRESENTATION).expect("parses");
    assert!(sd_jwt.has_key_binding(), "the Key Binding JWT is carried");
    assert_eq!(sd_jwt.disclosures().len(), 3);
    assert_eq!(processed(A3_PRESENTATION), json_of(A3_PROCESSED));
}

#[test]
fn appendix_a4_presentation_processes_to_the_payload_the_rfc_prints() {
    let sd_jwt = SdJwt::parse(A4_PRESENTATION).expect("parses");
    assert!(sd_jwt.has_key_binding());
    assert_eq!(processed(A4_PRESENTATION), json_of(A4_PROCESSED));
}

/// The RFC does not print this result, but it states how the SD-JWT was made:
/// every claim of the input, each hidden behind a Disclosure, plus the claims the
/// Issuer adds in the clear. Disclosing all of it gives the input back.
#[test]
fn appendix_a3_with_every_disclosure_gives_back_the_input_claims() {
    let mut expected = match json_of(A3_INPUT) {
        Value::Object(map) => map,
        other => panic!("the input is an object, got {other}"),
    };
    let Value::Object(shown) = json_of(A3_PROCESSED) else {
        panic!("the processed payload is an object");
    };
    for claim in ["iat", "exp", "cnf"] {
        expected.insert(claim.to_owned(), shown[claim].clone());
    }

    assert_eq!(processed(A3_ISSUED), Value::Object(expected));
}

#[test]
fn every_digest_the_rfc_prints_is_the_digest_of_its_disclosure() {
    let printed = S62_DISCLOSURES
        .iter()
        .map(|(name, digest, encoded)| (*name, *digest, *encoded))
        .chain([
            ("address", S61_ADDRESS.0, S61_ADDRESS.1),
            ("address", S63_ADDRESS.0, S63_ADDRESS.1),
        ]);
    for (name, digest, encoded) in printed {
        assert_eq!(digest_of(encoded), digest, "{name}");
        let parsed = disclosure(encoded);
        assert_eq!(parsed.claim_name(), name);
        assert_eq!(parsed.digest(), digest, "{name}");
    }
}

fn the_address() -> Value {
    json!({
        "street_address": "Schulstr. 12",
        "locality": "Schulpforta",
        "region": "Sachsen-Anhalt",
        "country": "DE"
    })
}

fn with_address(address: Value) -> Value {
    json!({
        "iss": "https://issuer.example.com",
        "iat": 1683000000,
        "exp": 1883000000,
        "sub": "6c5c0a49-b589-431d-bae7-219122a9ec2c",
        "address": address
    })
}

/// Section 6.1: the whole address is one claim, disclosed or not at all.
#[test]
fn section_6_1_the_address_is_disclosed_whole_or_not_at_all() {
    let shown = processed_with(S61_PAYLOAD, &[S61_ADDRESS.1]).expect("processes");
    assert_eq!(shown, with_address(the_address()));

    let hidden = processed_with(S61_PAYLOAD, &[]).expect("processes");
    let Value::Object(hidden) = hidden else {
        panic!("an object");
    };
    assert!(!hidden.contains_key("address"));
    assert!(!hidden.contains_key("_sd") && !hidden.contains_key("_sd_alg"));
}

/// Section 6.2: each member of the address is its own Disclosure, and a Holder
/// reveals as many as it chooses.
#[test]
fn section_6_2_each_member_of_the_address_is_disclosed_on_its_own() {
    let encoded: Vec<&str> = S62_DISCLOSURES.iter().map(|(_, _, d)| *d).collect();

    assert_eq!(
        processed_with(S62_PAYLOAD, &encoded).expect("processes"),
        with_address(the_address())
    );

    // Two of the four: the order the Disclosures arrive in is not the order of
    // the digests, and neither decides the result.
    let (country, locality) = (S62_DISCLOSURES[3].2, S62_DISCLOSURES[1].2);
    assert_eq!(
        processed_with(S62_PAYLOAD, &[country, locality]).expect("processes"),
        with_address(json!({ "locality": "Schulpforta", "country": "DE" }))
    );

    // None: the address is an empty object, which clause 7.1 step 3.e says to
    // write as `{}` and not to drop.
    assert_eq!(
        processed_with(S62_PAYLOAD, &[]).expect("processes"),
        with_address(json!({}))
    );
}

/// Section 6.2's second payload: `country` is in the clear, so there is no
/// Disclosure for it, and the other three are as before.
#[test]
fn section_6_2_a_member_left_in_the_clear_needs_no_disclosure() {
    let encoded: Vec<&str> = S62_DISCLOSURES[..3].iter().map(|(_, _, d)| *d).collect();
    assert_eq!(
        processed_with(S62_PAYLOAD_COUNTRY_IN_THE_CLEAR, &encoded).expect("processes"),
        with_address(the_address())
    );
    assert_eq!(
        processed_with(S62_PAYLOAD_COUNTRY_IN_THE_CLEAR, &[]).expect("processes"),
        with_address(json!({ "country": "DE" }))
    );
}

/// Section 6.3: the address is itself a Disclosure, and the Disclosure's value
/// holds the digests of the members, which are section 6.2's four.
#[test]
fn section_6_3_a_disclosure_that_holds_further_digests_is_opened_too() {
    let mut encoded = vec![S63_ADDRESS.1];
    encoded.extend(S62_DISCLOSURES.iter().map(|(_, _, d)| *d));
    assert_eq!(
        processed_with(S63_PAYLOAD, &encoded).expect("processes"),
        with_address(the_address())
    );

    // The address on its own: its members stay hidden, and the object they were
    // in is empty, not absent.
    assert_eq!(
        processed_with(S63_PAYLOAD, &[S63_ADDRESS.1]).expect("processes"),
        with_address(json!({}))
    );
}

/// Clause 7.1 step 5: a Disclosure no digest refers to is an error, however
/// legitimate it is on its own. Here one from section 6.2 rides along with
/// Appendix A.1's presentation, which has no digest for it.
#[test]
fn a_disclosure_the_token_does_not_reference_refuses_the_whole_token() {
    let mut sd_jwt = SdJwt::parse(A1_PRESENTATION).expect("parses");
    let mut disclosures = sd_jwt.disclosures().to_vec();
    disclosures.push(disclosure(S62_DISCLOSURES[0].2));
    sd_jwt = SdJwt::new(sd_jwt.jwt().to_owned(), disclosures);

    assert!(matches!(
        sd_jwt.disclosed_payload(),
        Err(SdJwtError::UnusedDisclosures(1))
    ));
}

/// Clause 7.1 step 3.c.ii.3: a claim name already present at the level of the
/// `_sd` key is an error. Section 6.2's second payload has `country` in the
/// clear, and its Disclosure for `country` would collide with it.
#[test]
fn a_disclosure_that_collides_with_a_claim_in_the_clear_is_refused() {
    let country = S62_DISCLOSURES[3];
    let mut payload = json_of(S62_PAYLOAD_COUNTRY_IN_THE_CLEAR);
    payload["address"]["_sd"]
        .as_array_mut()
        .expect("the address holds an `_sd` array")
        .push(json!(country.1));
    let payload = payload.to_string();

    assert!(matches!(
        processed_with(&payload, &[country.2]),
        Err(SdJwtError::ClaimCollision(name)) if name == "country"
    ));
}

#[test]
fn the_rfcs_payloads_are_objects_a_processed_payload_never_holds_the_mechanism() {
    for serialised in [A1_PRESENTATION, A3_PRESENTATION, A4_PRESENTATION, A3_ISSUED] {
        let Value::Object(map) = processed(serialised) else {
            panic!("an object");
        };
        assert!(
            !contains_key_anywhere(&map, "_sd") && !contains_key_anywhere(&map, "_sd_alg"),
            "clause 7.1 steps 3.e and 3.f remove both"
        );
    }
}

fn contains_key_anywhere(map: &Map<String, Value>, key: &str) -> bool {
    map.iter().any(|(k, v)| {
        k == key || match v {
            Value::Object(inner) => contains_key_anywhere(inner, key),
            Value::Array(items) => items.iter().any(
                |item| matches!(item, Value::Object(inner) if contains_key_anywhere(inner, key)),
            ),
            _ => false,
        }
    })
}

/// Clause 7.2: a Holder presents by choosing Disclosures, and what the Verifier
/// is left with is what the RFC prints. Appendix A.3 prints the issued SD-JWT and
/// the presentation made from it, so the presentation is rebuilt here from the
/// issued token by the digests of the Disclosures the RFC's Holder chose.
#[test]
fn a_holder_presenting_the_rfcs_choice_from_the_issued_token_gives_the_rfcs_presentation() {
    let issued = SdJwt::parse(A3_ISSUED).expect("parses");
    let chosen = SdJwt::parse(A3_PRESENTATION).expect("parses");
    let digests: Vec<String> = chosen
        .disclosures()
        .iter()
        .map(Disclosure::digest)
        .collect();
    let digests: Vec<&str> = digests.iter().map(String::as_str).collect();

    let presented = issued.present(&digests);

    let mut ours: Vec<&str> = presented
        .disclosures()
        .iter()
        .map(Disclosure::encoded)
        .collect();
    let mut theirs: Vec<&str> = chosen
        .disclosures()
        .iter()
        .map(Disclosure::encoded)
        .collect();
    // The RFC's Holder lists them in an order of its own; clause 7.2 sets none.
    ours.sort_unstable();
    theirs.sort_unstable();
    assert_eq!(ours, theirs);
    assert_eq!(presented.jwt(), chosen.jwt());
    assert_eq!(
        Value::Object(presented.disclosed_payload().expect("processes")),
        json_of(A3_PROCESSED)
    );
}

/// 🚨 A recorded deviation, not a behaviour to rely on. Clause 4.2.2 hides an array
/// element behind a placeholder `{"...": "<digest>"}`, and clause 7.1 step 3.d has
/// a Verifier remove every placeholder it has no Disclosure for. This module does
/// not read array-element Disclosures, so such a placeholder survives into the
/// processed payload as the object it is, where a consumer expecting an array of
/// values will find an object. The register lists it as a deviation. When array
/// elements are implemented this test fails, and the row is the next thing to edit.
#[test]
fn array_element_placeholders_survive_processing_unread() {
    let payload = json!({
        "iss": "https://issuer.example.com",
        "nationalities": [{ "...": "w0I8EKcdCtUPkGCNUrfwVp2xEgNjtoIDlOxc9-PlOhs" }, "DE"],
        "_sd_alg": "sha-256"
    });
    let shown = processed_with(&payload.to_string(), &[]).expect("processes");

    assert_eq!(
        shown["nationalities"],
        json!([{ "...": "w0I8EKcdCtUPkGCNUrfwVp2xEgNjtoIDlOxc9-PlOhs" }, "DE"]),
        "clause 7.1 step 3.d would leave only the elements that were in the clear"
    );
}

/// The other half of the same deviation: a two-element Disclosure, which is how
/// clause 4.2.2 spells an array element, is refused outright rather than applied.
#[test]
fn a_two_element_array_disclosure_is_refused_not_applied() {
    // ["lklxF5jMYlGTPUovMNIvCA", "FR"], as clause 4.2.2 forms one.
    let array_element = "WyJsa2x4RjVqTVlsR1RQVW92TU5JdkNBIiwgIkZSIl0";
    assert!(matches!(
        Disclosure::parse(array_element),
        Err(super::DisclosureError::NotATriple)
    ));
}
