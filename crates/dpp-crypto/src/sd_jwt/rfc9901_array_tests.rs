//! RFC 9901's examples that hide array elements, processed as a Verifier or Holder
//! would.
//!
//! Clause 4.2.2 defines a Disclosure for one element of an array, and clause 7.1
//! step 3 says what a Verifier does with the placeholder it stands for. These are
//! the RFC's own cases: the worked example of clauses 4.2.2 and 4.2.4.2 (the
//! second of three nationalities, hidden or shown), section 5's issuance and
//! presentation of two nationalities, and Appendix A.2, where the element that is
//! disclosed is an object holding digests of its own.
//!
//! None of it was produced by this code, so a reading of the clause shared by the
//! code and its tests would show up here: a placeholder left behind, an element
//! put in the wrong position, a Disclosure opened but not read for further
//! digests.
//!
//! As in `rfc9901_example_tests.rs`, no signature is checked. The RFC's are ES256.
//!
//! The RFC's data in this file is reused as Code Components under the IETF
//! Trust's Legal Provisions Relating to IETF Documents:
//!
//! > Copyright (c) 2025 IETF Trust and the persons identified as authors of the
//! > code. All rights reserved.
//! >
//! > Redistribution and use in source and binary forms, with or without
//! > modification, is permitted pursuant to, and subject to the license terms
//! > contained in, the Revised BSD License set forth in Section 4.c of the IETF
//! > Trust's Legal Provisions Relating to IETF Documents
//! > (<https://trustee.ietf.org/license-info>).

use serde_json::{Value, json};

use super::rfc9901_examples::{
    A2_PRESENTATION, A2_PROCESSED, S51_ARRAY_DISCLOSURES, S51_INPUT, S51_ISSUED, S52_PRESENTATION,
    S52_PROCESSED,
};
use super::tests::stub_jwt;
use super::{Disclosure, SdJwt, digest_of};

/// Clause 4.2.2: the Disclosure for the second element of `["DE", "FR", "US"]`.
const RFC_FR: &str = "WyJsa2x4RjVqTVlsR1RQVW92TU5JdkNBIiwgIkZSIl0";

/// Clause 4.2.4.2: the digest the RFC prints for it.
const RFC_FR_DIGEST: &str = "w0I8EKcdCtUPkGCNUrfwVp2xEgNjtoIDlOxc9-PlOhs";

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).expect("the RFC's JSON parses")
}

fn processed(serialised: &str) -> Value {
    let sd_jwt = SdJwt::parse(serialised).expect("the RFC's SD-JWT parses");
    Value::Object(
        sd_jwt
            .disclosed_payload()
            .expect("the RFC's SD-JWT processes"),
    )
}

#[test]
fn the_rfc_array_element_disclosure_hashes_to_its_published_digest() {
    let d = Disclosure::parse(RFC_FR).expect("the RFC's disclosure parses");
    assert!(d.is_array_element());
    assert_eq!(d.claim_name(), None);
    assert_eq!(d.salt(), "lklxF5jMYlGTPUovMNIvCA");
    assert_eq!(d.claim_value(), &json!("FR"));
    assert_eq!(d.digest(), RFC_FR_DIGEST);
    assert_eq!(digest_of(RFC_FR), RFC_FR_DIGEST);
}

/// Clause 4.2.4.2 states both outcomes in words: without the Disclosure the output
/// is `["DE", "US"]`, with it `["DE", "FR", "US"]`.
#[test]
fn the_worked_example_gives_the_two_outputs_the_rfc_states() {
    let payload = json!({
        "nationalities": ["DE", { "...": RFC_FR_DIGEST }, "US"]
    });

    let without = SdJwt::new(stub_jwt(&payload), vec![]);
    assert_eq!(
        Value::Object(without.disclosed_payload().expect("processes")),
        json!({ "nationalities": ["DE", "US"] })
    );

    let with = SdJwt::new(
        stub_jwt(&payload),
        vec![Disclosure::parse(RFC_FR).expect("parses")],
    );
    assert_eq!(
        Value::Object(with.disclosed_payload().expect("processes")),
        json!({ "nationalities": ["DE", "FR", "US"] })
    );
}

#[test]
fn every_array_digest_the_rfc_prints_is_the_digest_of_its_disclosure() {
    for ((digest, encoded), value) in S51_ARRAY_DISCLOSURES.iter().zip(["US", "DE"]) {
        assert_eq!(digest_of(encoded), *digest, "{value}");
        let parsed = Disclosure::parse(encoded).expect("parses");
        assert!(parsed.is_array_element());
        assert_eq!(parsed.claim_value(), &json!(value));
        assert_eq!(parsed.digest(), *digest);
    }
}

/// The RFC's presentation reveals one of two nationalities, and the Processed
/// payload it prints has one: the other placeholder is gone, not an object.
#[test]
fn section_5_2_presentation_processes_to_the_payload_the_rfc_prints() {
    let sd_jwt = SdJwt::parse(S52_PRESENTATION).expect("parses");
    assert!(sd_jwt.has_key_binding());
    assert_eq!(sd_jwt.disclosures().len(), 4);
    assert_eq!(
        sd_jwt
            .disclosures()
            .iter()
            .filter(|d| d.is_array_element())
            .count(),
        1
    );

    let shown = processed(S52_PRESENTATION);
    assert_eq!(shown, json_of(S52_PROCESSED));
    assert_eq!(shown["nationalities"], json!(["US"]));
}

/// The RFC does not print this result, but it states how the SD-JWT was made:
/// every claim of the input, each behind a Disclosure except `sub`, plus the claims
/// the Issuer adds in the clear. Disclosing all of it gives the input back, with
/// the nationalities in their original order.
#[test]
fn section_5_1_with_every_disclosure_gives_back_the_input_claims() {
    let Value::Object(mut expected) = json_of(S51_INPUT) else {
        panic!("the input is an object");
    };
    let Value::Object(shown) = json_of(S52_PROCESSED) else {
        panic!("the processed payload is an object");
    };
    for claim in ["iss", "iat", "exp", "cnf"] {
        expected.insert(claim.to_owned(), shown[claim].clone());
    }

    let all = processed(S51_ISSUED);
    assert_eq!(all, Value::Object(expected));
    assert_eq!(all["nationalities"], json!(["US", "DE"]));
}

/// Clause 7.2: from the issued token, a Holder that picks the RFC's four
/// Disclosures presents what section 5.2 prints. The element Disclosures are
/// chosen by digest like any other, so `present` needed no change for them.
#[test]
fn a_holder_presenting_the_rfcs_choice_gives_the_rfcs_presentation() {
    let issued = SdJwt::parse(S51_ISSUED).expect("parses");
    let chosen = SdJwt::parse(S52_PRESENTATION).expect("parses");
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
    ours.sort_unstable();
    theirs.sort_unstable();
    assert_eq!(ours, theirs);
    assert_eq!(
        Value::Object(presented.disclosed_payload().expect("processes")),
        json_of(S52_PROCESSED)
    );
}

/// Revealing the second element and not the first leaves the array with the second
/// alone: positions among what is shown follow the array, not the order the
/// Disclosures arrive in.
#[test]
fn revealing_either_nationality_alone_leaves_that_one() {
    let issued = SdJwt::parse(S51_ISSUED).expect("parses");
    let [(us, _), (de, _)] = S51_ARRAY_DISCLOSURES;

    for (digest, expected) in [(us, json!(["US"])), (de, json!(["DE"]))] {
        let shown = issued.present(&[digest]).disclosed_payload();
        assert_eq!(shown.expect("processes")["nationalities"], expected);
    }

    let none = issued.present(&[]).disclosed_payload().expect("processes");
    assert_eq!(none["nationalities"], json!([]));
}

/// Appendix A.2: the one `evidence` element is disclosed, its value is an object
/// holding digests of its own, and one of those is disclosed too. The Verifier
/// opens the placeholder and then reads what it opened.
#[test]
fn appendix_a2_presentation_processes_to_the_payload_the_rfc_prints() {
    let sd_jwt = SdJwt::parse(A2_PRESENTATION).expect("parses");
    assert!(
        sd_jwt
            .disclosures()
            .iter()
            .any(Disclosure::is_array_element),
        "the presentation carries an array-element Disclosure"
    );

    let shown = processed(A2_PRESENTATION);
    assert_eq!(shown, json_of(A2_PROCESSED));
    assert_eq!(
        shown["verified_claims"]["verification"]["evidence"],
        json!([{ "method": "pipp" }])
    );
}
