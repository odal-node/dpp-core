//! RFC 9901 mechanism tests.
//!
//! The properties here are the specification's, not a passport's — a passport
//! credential is `dpp-vc` and is tested there. What is pinned is that the
//! digests, salts and combined format behave as clause 4 and clause 7 say.

use serde_json::{Map, Value, json};

use super::builder::{SdJwt, build_payload, conceal};
use super::disclosure::{Disclosure, digest_of};
use super::error::{DisclosureError, SdJwtError};

/// Sign nothing — these tests exercise the disclosure mechanism, and the JWS
/// check is [`crate::jws::verifier`]'s. A stub JWT with a real payload is all
/// the mechanism needs, and using one keeps a key out of tests that do not
/// depend on signing.
fn stub_jwt(payload: &Value) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!(
        "{}.{}.{}",
        b64.encode(br#"{"alg":"EdDSA","typ":"dc+sd-jwt"}"#),
        b64.encode(payload.to_string()),
        b64.encode([0u8; 64])
    )
}

fn sample() -> Map<String, Value> {
    json!({
        "batteryChemistry": "LFP",
        "cathodeMaterial": ["LiFePO4"],
        "stateOfHealthPct": 87.5,
        "testReportResults": "report-4471",
        "safetyMeasures": ["Do not puncture"],
    })
    .as_object()
    .unwrap()
    .clone()
}

const HIDDEN: [&str; 4] = [
    "cathodeMaterial",
    "stateOfHealthPct",
    "testReportResults",
    "safetyMeasures",
];

fn issue() -> SdJwt {
    let (concealed, disclosures) = conceal(&sample(), |name| HIDDEN.contains(&name));
    let payload = build_payload(concealed, !disclosures.is_empty());
    SdJwt::new(stub_jwt(&payload), disclosures)
}

#[test]
fn concealed_claims_leave_no_cleartext_in_the_payload() {
    let (concealed, disclosures) = conceal(&sample(), |name| HIDDEN.contains(&name));

    assert_eq!(disclosures.len(), 4);
    assert!(concealed.contains_key("batteryChemistry"));
    for name in HIDDEN {
        assert!(!concealed.contains_key(name), "{name} left in cleartext");
    }
    let digests = concealed["_sd"].as_array().unwrap();
    assert_eq!(digests.len(), 4);
}

/// RFC 9901 clause 4.2.4.1: "The Issuer MUST hide the original order of the
/// claims in the array."
///
/// The failure this guards is silent and easy to write: pushing each digest as
/// the field is walked puts the array in source order, so a reader learns the
/// original structure from a token that discloses nothing.
#[test]
fn digest_order_does_not_follow_claim_order() {
    let (concealed, disclosures) = conceal(&sample(), |name| HIDDEN.contains(&name));

    let published: Vec<&str> = concealed["_sd"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let in_claim_order: Vec<String> = disclosures.iter().map(Disclosure::digest).collect();

    let mut sorted = in_claim_order.clone();
    sorted.sort();
    assert_eq!(published, sorted, "digests must be published sorted");

    // The sorted order must be a genuine reordering for at least one salt draw,
    // otherwise this test would pass on an implementation that does not sort.
    // Salts are random, so a single draw could coincide; repeat until one
    // differs, which is overwhelmingly the first iteration.
    let reordered = (0..32).any(|_| {
        let (c, d) = conceal(&sample(), |name| HIDDEN.contains(&name));
        let pub_order: Vec<String> = c["_sd"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        let claim_order: Vec<String> = d.iter().map(Disclosure::digest).collect();
        pub_order != claim_order
    });
    assert!(reordered, "sorting never reordered — suspect a no-op sort");
}

/// RFC 9901 clause 9.3: a new salt for each claim, 128 bits recommended.
#[test]
fn salts_are_unique_and_128_bits() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let (_, disclosures) = conceal(&sample(), |name| HIDDEN.contains(&name));

    let mut salts: Vec<&str> = disclosures.iter().map(Disclosure::salt).collect();
    let count = salts.len();
    salts.sort_unstable();
    salts.dedup();
    assert_eq!(salts.len(), count, "a salt was reused across claims");

    for d in &disclosures {
        assert_eq!(b64.decode(d.salt()).unwrap().len(), 16);
    }
}

/// Clause 9.3 again, in the direction that matters commercially: two issuances
/// of the same data must not be linkable by their digests.
#[test]
fn two_issuances_of_the_same_claims_share_no_digest() {
    let (first, _) = conceal(&sample(), |name| HIDDEN.contains(&name));
    let (second, _) = conceal(&sample(), |name| HIDDEN.contains(&name));

    let a: Vec<&Value> = first["_sd"].as_array().unwrap().iter().collect();
    let b: Vec<&Value> = second["_sd"].as_array().unwrap().iter().collect();
    assert!(
        a.iter().all(|d| !b.contains(d)),
        "a digest repeated across two issuances — salts are being reused"
    );
}

/// Clause 9.3's same-name-different-place requirement. A `$ref`'d definition
/// carries its disclosure class to every path that references it, so one leaf
/// name legitimately appears at several paths.
#[test]
fn the_same_claim_name_at_two_places_gets_two_salts() {
    let outer = json!({ "serialNumber": "A" }).as_object().unwrap().clone();
    let inner = json!({ "serialNumber": "A" }).as_object().unwrap().clone();

    let (_, a) = conceal(&outer, |n| n == "serialNumber");
    let (_, b) = conceal(&inner, |n| n == "serialNumber");

    assert_ne!(a[0].salt(), b[0].salt());
    assert_ne!(a[0].digest(), b[0].digest());
}

#[test]
fn a_full_credential_round_trips_through_the_combined_format() {
    let issued = issue();
    let parsed = SdJwt::parse(&issued.serialise()).unwrap();

    assert_eq!(parsed.disclosures().len(), 4);
    assert!(!parsed.has_key_binding());

    let payload = parsed.disclosed_payload().unwrap();
    assert_eq!(payload["batteryChemistry"], json!("LFP"));
    assert_eq!(payload["cathodeMaterial"], json!(["LiFePO4"]));
    assert_eq!(payload["stateOfHealthPct"], json!(87.5));
    assert!(!payload.contains_key("_sd"), "_sd survived processing");
    assert!(!payload.contains_key("_sd_alg"), "_sd_alg survived");
}

/// The merge-gate property, asserted over the presentation *bytes* rather than
/// inferred from the construction.
#[test]
fn a_two_of_four_presentation_does_not_carry_the_withheld_two() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let issued = issue();
    let presentation = issued.present(&["cathodeMaterial", "stateOfHealthPct"]);
    let wire = presentation.serialise();

    assert_eq!(presentation.disclosures().len(), 2);

    let payload = presentation.disclosed_payload().unwrap();
    assert_eq!(payload["cathodeMaterial"], json!(["LiFePO4"]));
    assert_eq!(payload["stateOfHealthPct"], json!(87.5));
    assert!(!payload.contains_key("testReportResults"));
    assert!(!payload.contains_key("safetyMeasures"));

    // Not in the wire bytes, and not in any base64url segment of them either.
    let decoded: String = wire
        .split('~')
        .flat_map(|s| s.split('.'))
        .filter_map(|s| b64.decode(s).ok())
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .collect::<Vec<_>>()
        .join("|");
    for needle in [
        "report-4471",
        "Do not puncture",
        "testReportResults",
        "safetyMeasures",
    ] {
        assert!(!wire.contains(needle), "{needle} is in the presentation");
        assert!(
            !decoded.contains(needle),
            "{needle} decodes out of a segment"
        );
    }

    // The issuer-signed JWT is forwarded unchanged, so the signature over it
    // still covers exactly what it covered at issuance.
    assert_eq!(presentation.jwt(), issued.jwt());
}

/// Tampering with a revealed value breaks its digest, and clause 7.1 step 4
/// then makes the whole credential unreadable rather than dropping the claim.
#[test]
fn a_tampered_disclosure_is_refused_not_ignored() {
    let issued = issue();
    let original = &issued.disclosures()[0];
    let forged = Disclosure::with_salt(
        original.salt().to_owned(),
        original.claim_name(),
        json!("forged"),
    );
    assert_ne!(forged.digest(), original.digest());

    let tampered = SdJwt::new(issued.jwt().to_owned(), vec![forged]);
    assert_eq!(
        tampered.disclosed_payload(),
        Err(SdJwtError::UnusedDisclosures(1))
    );
}

#[test]
fn a_disclosure_naming_a_cleartext_claim_is_refused() {
    let issued = issue();
    let collides = Disclosure::new("batteryChemistry", json!("NMC"));
    // Put its digest in the token so it is *found*, which is what isolates the
    // collision check from the unused-disclosure one.
    let payload = json!({
        "batteryChemistry": "LFP",
        "_sd": [collides.digest()],
        "_sd_alg": "sha-256",
    });
    let sd = SdJwt::new(stub_jwt(&payload), vec![collides]);
    assert_eq!(
        sd.disclosed_payload(),
        Err(SdJwtError::ClaimCollision("batteryChemistry".to_owned()))
    );
    let _ = issued;
}

#[test]
fn a_present_but_unsupported_sd_alg_is_refused_rather_than_assumed() {
    let payload = json!({ "_sd": [], "_sd_alg": "sha-512" });
    let sd = SdJwt::new(stub_jwt(&payload), vec![]);
    assert_eq!(
        sd.disclosed_payload(),
        Err(SdJwtError::UnsupportedHashAlg("sha-512".to_owned()))
    );
}

#[test]
fn an_absent_sd_alg_means_sha_256() {
    let d = Disclosure::new("x", json!(1));
    let payload = json!({ "_sd": [d.digest()] });
    let sd = SdJwt::new(stub_jwt(&payload), vec![d]);
    assert_eq!(sd.disclosed_payload().unwrap()["x"], json!(1));
}

#[test]
fn nothing_concealed_means_no_sd_claim() {
    let (concealed, disclosures) = conceal(&sample(), |_| false);
    assert!(disclosures.is_empty());
    assert!(!concealed.contains_key("_sd"));
    let payload = build_payload(concealed, false);
    assert!(payload.get("_sd_alg").is_none());
}

#[test]
fn a_disclosure_survives_a_parse_and_reencode_unchanged() {
    let d = Disclosure::new("cathodeMaterial", json!(["LiFePO4"]));
    let back = Disclosure::parse(d.encoded()).unwrap();
    assert_eq!(back, d);
    assert_eq!(back.digest(), d.digest());
}

#[test]
fn reserved_claim_names_are_refused() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    for name in ["_sd", "..."] {
        let encoded = b64.encode(json!(["salt", name, 1]).to_string());
        assert_eq!(
            Disclosure::parse(&encoded),
            Err(DisclosureError::ReservedClaimName)
        );
    }
}

#[test]
fn a_two_element_array_disclosure_is_refused() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let encoded = b64.encode(json!(["salt", "value"]).to_string());
    assert_eq!(
        Disclosure::parse(&encoded),
        Err(DisclosureError::NotATriple)
    );
}

#[test]
fn a_token_without_a_trailing_separator_is_not_an_sd_jwt() {
    assert!(matches!(
        SdJwt::parse("onlyonesegment"),
        Err(SdJwtError::NotAnSdJwt)
    ));
}

#[test]
fn a_key_binding_segment_is_seen_and_not_read_as_a_disclosure() {
    let issued = issue();
    let with_kb = format!("{}kb-jwt-placeholder", issued.serialise());
    let parsed = SdJwt::parse(&with_kb).unwrap();
    assert_eq!(parsed.disclosures().len(), 4);
    assert!(parsed.has_key_binding());
}

#[test]
fn digest_of_matches_the_disclosures_own_digest() {
    let d = Disclosure::new("x", json!("y"));
    assert_eq!(digest_of(d.encoded()), d.digest());
}
