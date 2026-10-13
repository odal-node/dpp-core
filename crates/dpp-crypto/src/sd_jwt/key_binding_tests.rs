//! Key Binding, RFC 9901 clauses 4.1.2, 4.3 and 7.3.
//!
//! The Issuer-signed JWT here is a stub, as in this module's other tests: the
//! Issuer's signature is the caller's to check. The Key Binding JWT is signed for
//! real, with a Holder key from a key store, because its signature is part of
//! what clause 7.3 asks this crate to check.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde_json::{Map, Value, json};

use super::rfc9901_examples::{A3_PRESENTATION, A4_PRESENTATION, S52_PRESENTATION};
use super::tests::stub_jwt;
use super::{
    KeyBinding, KeyBindingPolicy, SdJwt, SdJwtError, build_payload, conceal,
    holder_key_confirmation,
};
use crate::jws::signer::sign_with_header;
use crate::keystore::KeyEntry;
use crate::test_support::temp_store;

const NOW: i64 = 1_800_000_000;

fn holder(label: &str) -> KeyEntry {
    temp_store(label, "holder")
        .load_key("holder")
        .expect("the key was just made")
}

fn binding() -> KeyBinding {
    KeyBinding {
        audience: "https://verifier.example".to_owned(),
        nonce: "n-0S6_WzA2Mj".to_owned(),
        issued_at: NOW,
    }
}

fn policy() -> KeyBindingPolicy {
    KeyBindingPolicy {
        audience: "https://verifier.example".to_owned(),
        nonce: "n-0S6_WzA2Mj".to_owned(),
        now: NOW,
        max_age: 300,
        leeway: 60,
    }
}

/// A token whose Issuer named `cnf`, with `given_name` and `family_name` hidden.
fn issued_to(cnf: Value) -> SdJwt {
    let mut claims = json!({
        "iss": "https://issuer.example",
        "given_name": "Erika",
        "family_name": "Mustermann",
    })
    .as_object()
    .unwrap()
    .clone();
    claims.insert("cnf".to_owned(), cnf);
    let (kept, disclosures) = conceal(&claims, |n| n.ends_with("_name")).unwrap();
    SdJwt::new(stub_jwt(&build_payload(kept, true)), disclosures)
}

fn issued(holder: &KeyEntry) -> SdJwt {
    issued_to(holder_key_confirmation(&holder.verifying_key.to_bytes()))
}

fn every_digest(sd_jwt: &SdJwt) -> Vec<String> {
    sd_jwt.disclosures().iter().map(|d| d.digest()).collect()
}

fn decoded(jwt: &str, part: usize) -> Map<String, Value> {
    let bytes = B64.decode(jwt.split('.').nth(part).unwrap()).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// `presentation` with its Key Binding JWT replaced by one signed by `key` over
/// `header` and `claims`, so a test can break one thing at a time.
fn rebound(presentation: &SdJwt, key: &KeyEntry, header: Value, claims: Value) -> SdJwt {
    let header = header.as_object().unwrap().clone();
    let kb = sign_with_header(key, &header, claims.to_string().as_bytes());
    let unbound = presentation.serialise();
    let (sd_jwt, _) = unbound.rsplit_once('~').unwrap();
    SdJwt::parse(&format!("{sd_jwt}~{kb}")).unwrap()
}

fn good_claims(presentation: &SdJwt) -> Value {
    json!({
        "iat": NOW,
        "aud": "https://verifier.example",
        "nonce": "n-0S6_WzA2Mj",
        "sd_hash": presentation.sd_hash(),
    })
}

fn kb_header() -> Value {
    json!({ "alg": "EdDSA", "typ": "kb+jwt" })
}

#[test]
fn the_issuer_names_the_holder_key_with_the_jwk_confirmation_method() {
    let key = [7u8; 32];
    assert_eq!(
        holder_key_confirmation(&key),
        json!({ "jwk": { "kty": "OKP", "crv": "Ed25519", "x": B64.encode(key) } })
    );
}

#[test]
fn a_bound_presentation_verifies_and_reads_to_what_was_chosen() {
    let key = holder("kb-roundtrip");
    let sd_jwt = issued(&key);
    let given = sd_jwt.digests_for_claim("given_name");
    let presented = sd_jwt
        .present_with_key_binding(&[given[0].as_str()], &binding(), &key)
        .expect("binds");

    let payload = presented
        .disclosed_payload_with_key_binding(&policy())
        .expect("verifies");
    assert_eq!(payload["given_name"], json!("Erika"));
    assert!(!payload.contains_key("family_name"));

    // Clause 4.3: the header says what the JWT is and how it is signed, and no
    // more; the payload is the four claims.
    let kb = presented.serialise().rsplit_once('~').unwrap().1.to_owned();
    assert_eq!(
        Value::Object(decoded(&kb, 0)),
        json!({ "alg": "EdDSA", "typ": "kb+jwt" })
    );
    assert_eq!(
        Value::Object(decoded(&kb, 1)),
        good_claims(&presented),
        "iat, aud, nonce and the sd_hash of what was presented"
    );
}

/// Clause 4.3.1, on the RFC's own data: the `sd_hash` each of its three Key
/// Binding JWTs carries is the digest of the presentation it is attached to. The
/// RFC's Holder keys are P-256, so their signatures are not this crate's to check.
#[test]
fn the_rfc_s_key_binding_jwts_carry_the_sd_hash_this_crate_computes() {
    for presentation in [A3_PRESENTATION, A4_PRESENTATION, S52_PRESENTATION] {
        let sd_jwt = SdJwt::parse(presentation).expect("parses");
        let (_, kb) = presentation.rsplit_once('~').unwrap();
        assert_eq!(
            decoded(kb, 1)["sd_hash"],
            json!(sd_jwt.sd_hash()),
            "the sd_hash of {}",
            &presentation[..24]
        );
    }
}

#[test]
fn a_token_without_a_key_binding_jwt_is_refused_when_one_is_required() {
    let key = holder("kb-missing");
    let presented = issued(&key).present(&[]).unwrap();
    assert_eq!(
        presented.disclosed_payload_with_key_binding(&policy()),
        Err(SdJwtError::KeyBindingMissing)
    );
}

#[test]
fn a_holder_key_must_be_named_and_be_ed25519() {
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
    let mut declared = holder_key_confirmation(&key.verifying_key.to_bytes());
    declared["jwk"]["alg"] = json!("ES256");
    assert_eq!(
        issued_to(declared)
            .present_with_key_binding(&[], &binding(), &key)
            .err(),
        Some(SdJwtError::UnsupportedHolderKey)
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
    claims.insert(
        "cnf".to_owned(),
        holder_key_confirmation(&key.verifying_key.to_bytes()),
    );
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

/// Each check of clause 7.3 step 5, broken alone.
#[test]
fn each_check_of_step_5_refuses_on_its_own() {
    let key = holder("kb-checks");
    let stranger = holder("kb-stranger");
    let presented = issued(&key)
        .present_with_key_binding(&[], &binding(), &key)
        .unwrap();
    let claims = good_claims(&presented);
    let with = |field: &str, value: Value| {
        let mut c = claims.clone();
        c[field] = value;
        c
    };
    let without = |field: &str| {
        let mut c = claims.clone();
        c.as_object_mut().unwrap().remove(field);
        c
    };
    let rejected = |sd_jwt: SdJwt| sd_jwt.disclosed_payload_with_key_binding(&policy()).err();
    let refused = |why: &'static str| Some(SdJwtError::KeyBindingRejected(why));

    // The control: rebuilt the same way, it verifies.
    assert_eq!(
        rejected(rebound(&presented, &key, kb_header(), claims.clone())),
        None
    );

    for (sd_jwt, why) in [
        (
            rebound(&presented, &stranger, kb_header(), claims.clone()),
            "the signature",
        ),
        (
            rebound(
                &presented,
                &key,
                json!({ "alg": "none", "typ": "kb+jwt" }),
                claims.clone(),
            ),
            "the signature",
        ),
        (
            rebound(
                &presented,
                &key,
                json!({ "alg": "EdDSA", "typ": "kb+jwt", "crit": ["x"], "x": 1 }),
                claims.clone(),
            ),
            "the signature",
        ),
        (
            rebound(
                &presented,
                &key,
                json!({ "alg": "EdDSA", "typ": "jwt" }),
                claims.clone(),
            ),
            "typ",
        ),
        (
            rebound(&presented, &key, json!({ "alg": "EdDSA" }), claims.clone()),
            "typ",
        ),
        (
            rebound(&presented, &key, kb_header(), with("iat", json!(NOW - 301))),
            "iat",
        ),
        (
            rebound(&presented, &key, kb_header(), with("iat", json!(NOW + 61))),
            "iat",
        ),
        (
            rebound(&presented, &key, kb_header(), with("iat", json!("now"))),
            "iat",
        ),
        (
            rebound(&presented, &key, kb_header(), without("iat")),
            "iat",
        ),
        (
            rebound(
                &presented,
                &key,
                kb_header(),
                with("nonce", json!("replayed")),
            ),
            "nonce",
        ),
        (
            rebound(&presented, &key, kb_header(), without("nonce")),
            "nonce",
        ),
        (
            rebound(
                &presented,
                &key,
                kb_header(),
                with("aud", json!("https://elsewhere.example")),
            ),
            "aud",
        ),
        (
            rebound(
                &presented,
                &key,
                kb_header(),
                with("aud", json!(["https://verifier.example"])),
            ),
            "aud",
        ),
        (
            rebound(
                &presented,
                &key,
                kb_header(),
                with("sd_hash", json!("not-the-hash")),
            ),
            "sd_hash",
        ),
        (
            rebound(&presented, &key, kb_header(), without("sd_hash")),
            "sd_hash",
        ),
        (
            rebound(&presented, &key, kb_header(), with("exp", json!(NOW - 61))),
            "exp",
        ),
        (
            rebound(&presented, &key, kb_header(), with("nbf", json!(NOW + 61))),
            "nbf",
        ),
    ] {
        assert_eq!(rejected(sd_jwt), refused(why), "{why}");
    }
}

/// Clause 4.3.1: the hash is over this presentation's own Disclosures, so a Key
/// Binding JWT lifted from one presentation does not bind another.
#[test]
fn a_key_binding_jwt_moved_to_another_selection_is_refused() {
    let key = holder("kb-moved");
    let sd_jwt = issued(&key);
    let digests = every_digest(&sd_jwt);
    let one = sd_jwt
        .present_with_key_binding(&[digests[0].as_str()], &binding(), &key)
        .unwrap();
    let both = sd_jwt
        .present(&[digests[0].as_str(), digests[1].as_str()])
        .unwrap();
    let (_, kb) = one
        .serialise()
        .rsplit_once('~')
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .unwrap();
    let moved = SdJwt::parse(&format!("{}{kb}", both.serialise())).unwrap();
    assert_eq!(
        moved.disclosed_payload_with_key_binding(&policy()),
        Err(SdJwtError::KeyBindingRejected("sd_hash"))
    );
}

/// RFC 7515 clause 4.1.9: a `typ` is a media type, compared without regard to
/// case and read with `application/` before it when it has no `/`.
#[test]
fn the_type_is_compared_as_a_media_type() {
    let key = holder("kb-typ");
    let presented = issued(&key)
        .present_with_key_binding(&[], &binding(), &key)
        .unwrap();
    for typ in ["kb+jwt", "KB+JWT", "application/kb+jwt"] {
        let header = json!({ "alg": "EdDSA", "typ": typ });
        assert!(
            rebound(&presented, &key, header, good_claims(&presented))
                .disclosed_payload_with_key_binding(&policy())
                .is_ok(),
            "{typ}"
        );
    }
}

/// A Holder key that declares its algorithm holds the signature to it.
#[test]
fn a_declared_algorithm_is_the_one_written_and_held_to() {
    let key = holder("kb-declared");
    let mut cnf = holder_key_confirmation(&key.verifying_key.to_bytes());
    cnf["jwk"]["alg"] = json!("Ed25519");
    let presented = issued_to(cnf)
        .present_with_key_binding(&[], &binding(), &key)
        .unwrap();
    let kb = presented.serialise().rsplit_once('~').unwrap().1.to_owned();
    assert_eq!(decoded(&kb, 0)["alg"], json!("Ed25519"));
    assert!(
        presented
            .disclosed_payload_with_key_binding(&policy())
            .is_ok()
    );

    let other = rebound(&presented, &key, kb_header(), good_claims(&presented));
    assert_eq!(
        other.disclosed_payload_with_key_binding(&policy()),
        Err(SdJwtError::KeyBindingRejected("alg"))
    );
}

/// Clause 7.3 step 1: whether Key Binding is checked is the caller's policy. A
/// caller that does not require it reads a bound token as it reads any other.
#[test]
fn a_verifier_that_does_not_require_key_binding_reads_the_token_as_before() {
    let key = holder("kb-not-required");
    let presented = issued(&key)
        .present_with_key_binding(&[], &binding(), &key)
        .unwrap();
    assert!(presented.has_key_binding());
    assert!(presented.disclosed_payload().is_ok());
}
