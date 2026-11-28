//! RFC 9901's examples are signed with ES256: the Issuer with the key of
//! Appendix A.5, each Holder with the P-256 key in its `cnf`. Every signature they
//! carry is checked here as a third-party vector, and the Key Binding path runs
//! end to end on them, signature included.

use base64::Engine;
use serde_json::{Value, json};

use super::super::super::builder::SdJwt;
use super::super::super::key_binding::KeyBindingPolicy;
use super::super::common::{
    A1_PRESENTATION, A2_PRESENTATION, A3_ISSUED, A3_PRESENTATION, A4_PRESENTATION, S51_ISSUED,
    S52_PRESENTATION,
};
use crate::jws::public_key::PublicKey;
use crate::jws::verifier::verify_jws;

/// Appendix A.5: the key that validates every Issuer signature in the examples.
fn issuer_key() -> PublicKey {
    PublicKey::from_jwk(&json!({
        "kty": "EC",
        "crv": "P-256",
        "x": "b28d4MwZMjw8-00CG4xfnn9SLMVMM19SlqZpVb_uNtQ",
        "y": "Xv5zWwuoaTgdS6hV43yI6gBwTnjukmFQQnJ_kCxzqk8",
    }))
    .expect("the RFC's issuer key")
}

#[test]
fn every_issuer_signature_in_the_examples_verifies_under_the_appendix_a5_key() {
    for (name, token) in [
        ("A.1", A1_PRESENTATION),
        ("A.2", A2_PRESENTATION),
        ("A.3 issued", A3_ISSUED),
        ("A.3 presented", A3_PRESENTATION),
        ("A.4", A4_PRESENTATION),
        ("5.1", S51_ISSUED),
        ("5.2", S52_PRESENTATION),
    ] {
        let sd_jwt = SdJwt::parse(token).expect(name);
        verify_jws(sd_jwt.jwt(), &issuer_key()).expect("well-formed");
    }
}

/// The claims of the Key Binding JWT a presentation ends with.
fn key_binding_claims(presentation: &str) -> Value {
    let kb = presentation.rsplit('~').next().expect("a KB-JWT");
    let claims = kb.split('.').nth(1).expect("claims");
    serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(claims)
            .expect("base64url"),
    )
    .expect("json")
}

/// The whole Key Binding check of clause 7.3, on the RFC's presentations: the
/// Holder key comes from `cnf` (a P-256 JWK), the KB-JWT is ES256 under it, and
/// `aud`, `nonce`, `iat` and `sd_hash` are the RFC's.
#[test]
fn the_rfc_presentations_pass_key_binding_with_their_p256_holder_keys() {
    for (name, presentation) in [
        ("A.3", A3_PRESENTATION),
        ("A.4", A4_PRESENTATION),
        ("5.2", S52_PRESENTATION),
    ] {
        let claims = key_binding_claims(presentation);
        let policy = KeyBindingPolicy {
            audience: claims["aud"].as_str().expect("aud").to_owned(),
            nonce: claims["nonce"].as_str().expect("nonce").to_owned(),
            now: claims["iat"].as_i64().expect("iat"),
            max_age: 300,
            leeway: 0,
        };
        let sd_jwt = SdJwt::parse(presentation).expect(name);
        let payload = sd_jwt
            .disclosed_payload_with_key_binding(&policy)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(payload["cnf"]["jwk"]["kty"], "EC", "{name}");

        // Another Verifier's nonce is refused: the signature alone is not enough.
        let mut other = policy.clone();
        other.nonce.push('x');
        assert!(
            sd_jwt.disclosed_payload_with_key_binding(&other).is_err(),
            "{name}"
        );
    }
}
