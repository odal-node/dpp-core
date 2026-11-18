//! The Holder key an Issuer names in `cnf` (RFC 9901 clause 4.1.2, carried by the
//! `jwk` confirmation method of RFC 7800 §3.2). This crate reads an Ed25519,
//! P-256, P-384 or P-521 key there, which must be a point on its curve, and an `alg` the JWK
//! declares only when it is the key's own. A Holder binds a presentation only with
//! the key the token names.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde_json::{Map, Value, json};

use super::key_binding_tests::{
    binding, decoded, every_digest, good_claims, holder, issued, issued_to, policy, rebound,
};
use super::tests::stub_jwt;
use super::{SdJwt, SdJwtError, build_payload, conceal, holder_key_confirmation};
use crate::keystore::KeyStore;
use crate::test_support::temp_path;

/// The public keys of RFC 8032 clause 7.1's TEST 1 and TEST 3: published Ed25519
/// keys, not generated here.
const TEST1_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const TEST3_PUBLIC: &str = "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025";

#[test]
fn the_issuer_names_the_holder_key_with_the_jwk_confirmation_method() {
    let raw = hex::decode(TEST3_PUBLIC).expect("hex");
    let key = crate::jws::PublicKey::from_bytes(crate::jws::KeyAlgorithm::Ed25519, &raw)
        .expect("a published Ed25519 key");
    assert_eq!(
        holder_key_confirmation(&key),
        json!({ "jwk": { "kty": "OKP", "crv": "Ed25519", "x": B64.encode(key.as_bytes()) } })
    );
}

/// A Holder of `algorithm` (P-256, P-384 or P-521), named in `cnf` with its JOSE
/// `alg` declared: the Key Binding JWT is signed under that key and declares the
/// same `alg`, and the Verifier reads the payload back as it does for an Ed25519
/// Holder.
fn an_ec_holder_round_trips_through_key_binding(algorithm: crate::jws::KeyAlgorithm) {
    let store = KeyStore::open(temp_path("kb-ec-holder"), "test-pass").expect("open");
    let key = store
        .generate_key_with("holder", algorithm)
        .expect("generate");
    let mut cnf = holder_key_confirmation(&key.public_key());
    cnf["jwk"]["alg"] = json!(algorithm.jose_alg());
    let sd_jwt = issued_to(cnf);
    let given = sd_jwt.digests_for_claim("given_name");
    let presented = sd_jwt
        .present_with_key_binding(&[given[0].as_str()], &binding(), &key)
        .expect("binds");

    let payload = presented
        .disclosed_payload_with_key_binding(&policy())
        .expect("verifies");
    assert_eq!(payload["given_name"], json!("Erika"));
    assert!(!payload.contains_key("family_name"));

    let kb = presented.serialise().rsplit_once('~').unwrap().1.to_owned();
    assert_eq!(
        Value::Object(decoded(&kb, 0)),
        json!({ "alg": algorithm.jose_alg(), "typ": "kb+jwt" })
    );
    assert_eq!(
        Value::Object(decoded(&kb, 1)),
        good_claims(&presented),
        "iat, aud, nonce and the sd_hash of what was presented"
    );
}

/// A P-256 Holder, named in `cnf` with its `alg` declared: the Key Binding JWT is
/// ES256 under that key, and the Verifier reads the payload back as it does for an
/// Ed25519 Holder.
#[test]
fn a_p256_holder_round_trips_through_key_binding() {
    assert_eq!(crate::jws::KeyAlgorithm::EcdsaP256.jose_alg(), "ES256");
    an_ec_holder_round_trips_through_key_binding(crate::jws::KeyAlgorithm::EcdsaP256);
}

/// P-384 and P-521 Holders bind as the P-256 one does: each declares its own `alg`
/// in `cnf` (ES384 and ES512), and the Key Binding JWT is held to it.
#[test]
fn p384_and_p521_holders_round_trip_through_key_binding() {
    assert_eq!(crate::jws::KeyAlgorithm::EcdsaP384.jose_alg(), "ES384");
    assert_eq!(crate::jws::KeyAlgorithm::EcdsaP521.jose_alg(), "ES512");
    an_ec_holder_round_trips_through_key_binding(crate::jws::KeyAlgorithm::EcdsaP384);
    an_ec_holder_round_trips_through_key_binding(crate::jws::KeyAlgorithm::EcdsaP521);
}

/// A P-256 Holder key whose JWK declares `EdDSA` names no key a signature can be
/// held to. The Holder is refused before anything is signed, and a Verifier refuses
/// the same `cnf` in a token that carries a Key Binding JWT.
#[test]
fn a_p256_holder_key_that_declares_eddsa_is_refused() {
    let store = KeyStore::open(temp_path("kb-p256-eddsa"), "test-pass").expect("open");
    let key = store
        .generate_key_with("holder", crate::jws::KeyAlgorithm::EcdsaP256)
        .expect("generate");
    let mut cnf = holder_key_confirmation(&key.public_key());
    cnf["jwk"]["alg"] = json!("EdDSA");
    let sd_jwt = issued_to(cnf);
    assert_eq!(
        sd_jwt.present_with_key_binding(&[], &binding(), &key).err(),
        Some(SdJwtError::UnsupportedHolderKey)
    );

    let presented = sd_jwt.present(&[]).expect("presents");
    let bound = rebound(
        &presented,
        &key,
        json!({ "alg": "ES256", "typ": "kb+jwt" }),
        good_claims(&presented),
    );
    assert_eq!(
        bound.disclosed_payload_with_key_binding(&policy()),
        Err(SdJwtError::UnsupportedHolderKey)
    );
}

/// An EC Holder key whose JWK declares another curve's `alg` names no key that `alg`
/// can be held to. The Holder is refused before anything is signed, and a Verifier
/// refuses the same `cnf` in a token that carries a Key Binding JWT.
#[test]
fn an_ec_holder_key_that_declares_another_curves_alg_is_refused() {
    let store = KeyStore::open(temp_path("kb-ec-other-alg"), "test-pass").expect("open");
    for (name, algorithm, own_alg, declared) in [
        (
            "p384",
            crate::jws::KeyAlgorithm::EcdsaP384,
            "ES384",
            "ES256",
        ),
        (
            "p521",
            crate::jws::KeyAlgorithm::EcdsaP521,
            "ES512",
            "ES384",
        ),
    ] {
        let key = store.generate_key_with(name, algorithm).expect("generate");
        let mut cnf = holder_key_confirmation(&key.public_key());
        cnf["jwk"]["alg"] = json!(declared);
        let sd_jwt = issued_to(cnf);
        assert_eq!(
            sd_jwt.present_with_key_binding(&[], &binding(), &key).err(),
            Some(SdJwtError::UnsupportedHolderKey),
            "a {name} key declaring {declared}"
        );

        let presented = sd_jwt.present(&[]).expect("presents");
        let bound = rebound(
            &presented,
            &key,
            json!({ "alg": own_alg, "typ": "kb+jwt" }),
            good_claims(&presented),
        );
        assert_eq!(
            bound.disclosed_payload_with_key_binding(&policy()),
            Err(SdJwtError::UnsupportedHolderKey),
            "a {name} key declaring {declared}"
        );
    }
}

#[test]
fn a_holder_key_must_be_named_and_of_a_supported_curve() {
    let key = holder("kb-no-cnf");
    let unnamed = issued_to(json!(null));
    assert_eq!(
        unnamed
            .present_with_key_binding(&[], &binding(), &key)
            .err(),
        Some(SdJwtError::NoHolderKey)
    );
    let p256 = issued_to(json!({ "jwk": { "kty": "EC", "crv": "P-256", "x": "AA", "y": "AA" } }));
    assert_eq!(
        p256.present_with_key_binding(&[], &binding(), &key).err(),
        Some(SdJwtError::UnsupportedHolderKey)
    );
    let mut declared = holder_key_confirmation(&key.public_key());
    declared["jwk"]["alg"] = json!("ES256");
    assert_eq!(
        issued_to(declared)
            .present_with_key_binding(&[], &binding(), &key)
            .err(),
        Some(SdJwtError::UnsupportedHolderKey)
    );
}

/// A Holder key is named by its JWK, and a JWK whose `x` is not a point on the
/// curve names no key, so the Holder is refused before anything is signed. The
/// control is the same token naming the real key, which binds.
#[test]
fn a_holder_key_that_is_not_a_point_on_the_curve_is_refused() {
    let key = holder("kb-off-curve");
    let mut off_curve = hex::decode(TEST1_PUBLIC).expect("hex");
    off_curve[0] ^= 1;
    let cnf = json!({ "jwk": { "kty": "OKP", "crv": "Ed25519", "x": B64.encode(&off_curve) } });

    assert_eq!(
        issued_to(cnf)
            .present_with_key_binding(&[], &binding(), &key)
            .err(),
        Some(SdJwtError::UnsupportedHolderKey)
    );
    assert!(
        issued(&key)
            .present_with_key_binding(&[], &binding(), &key)
            .is_ok()
    );
}

#[test]
fn a_holder_cannot_bind_with_a_key_the_token_does_not_name() {
    let named = holder("kb-named");
    let other = holder("kb-other");
    assert_eq!(
        issued(&named)
            .present_with_key_binding(&[], &binding(), &other)
            .err(),
        Some(SdJwtError::HolderKeyMismatch)
    );
}

/// A concealed `cnf` that is not chosen leaves the Verifier no key to check with,
/// so the Holder is told now rather than every Verifier later.
#[test]
fn a_concealed_holder_key_has_to_be_presented() {
    let key = holder("kb-concealed-cnf");
    let mut claims = Map::new();
    claims.insert("cnf".to_owned(), holder_key_confirmation(&key.public_key()));
    let (kept, disclosures) = conceal(&claims, |_| true).unwrap();
    let sd_jwt = SdJwt::new(stub_jwt(&build_payload(kept, true)), disclosures);
    assert_eq!(
        sd_jwt.present_with_key_binding(&[], &binding(), &key).err(),
        Some(SdJwtError::NoHolderKey)
    );
    let cnf = every_digest(&sd_jwt);
    let presented = sd_jwt
        .present_with_key_binding(&[cnf[0].as_str()], &binding(), &key)
        .expect("binds once cnf travels");
    assert!(
        presented
            .disclosed_payload_with_key_binding(&policy())
            .is_ok()
    );
}

/// RFC 7800 §3.1 allows one proof-of-possession key in `cnf`, so `jwk` is
/// refused beside `jku`, which names the key by a URL instead.
#[test]
fn a_jwk_beside_a_jku_is_refused() {
    let key = holder("kb-jwk-jku");
    let mut cnf = holder_key_confirmation(&key.public_key());
    cnf["jku"] = json!("https://keys.example/holder.jwk");
    assert_eq!(
        issued_to(cnf)
            .present_with_key_binding(&[], &binding(), &key)
            .err(),
        Some(SdJwtError::UnsupportedHolderKey)
    );
}

/// RFC 7800 §3.1 allows one proof-of-possession key in `cnf`, so `jwk` is
/// refused beside `jwe`, which carries the key encrypted.
#[test]
fn a_jwk_beside_a_jwe_is_refused() {
    let key = holder("kb-jwk-jwe");
    let mut cnf = holder_key_confirmation(&key.public_key());
    cnf["jwe"] = json!("eyJhbGciOiJSU0EtT0FFUCJ9.x.y.z.w");
    assert_eq!(
        issued_to(cnf)
            .present_with_key_binding(&[], &binding(), &key)
            .err(),
        Some(SdJwtError::UnsupportedHolderKey)
    );
}

/// `kid` and other members that RFC 7800 does not limit to one stay valid beside `jwk`.
#[test]
fn a_jwk_beside_an_unrelated_member_is_accepted() {
    let key = holder("kb-jwk-extra");
    let mut cnf = holder_key_confirmation(&key.public_key());
    cnf["example-extension"] = json!("not a confirmation method");
    assert!(
        issued_to(cnf)
            .present_with_key_binding(&[], &binding(), &key)
            .is_ok()
    );
}
