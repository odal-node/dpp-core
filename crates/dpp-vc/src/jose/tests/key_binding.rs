//! Key Binding of an SD-JWT, RFC 9901 §7.3 and §9.5: the verifier decides whether it is
//! required, the choice never depends on whether a Key Binding JWT arrived, and, when it is
//! required, the checks of step 5 apply to the Holder key in the `cnf` of the processed payload.
//!
//! Tokens are made as a deployment makes them: the signer writes the Holder's key into `cnf`,
//! and the Holder binds a presentation with `SdJwt::present_with_key_binding`.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::{ClaimPath, KeyBinding, KeyBindingPolicy, SdJwt, holder_key_confirmation};

use super::super::{
    JoseError, KeyBindingRequirement, VerifyOptions, sign_credential, sign_credential_sd_jwt,
    sign_presentation, sign_presentation_sd_jwt, verify_credential_sd_jwt, verify_presentation,
    verify_presentation_sd_jwt,
};
use crate::test_support::{
    KID, NOT_REQUIRED, credential, envelope, forge, generated, presentation, sd_envelope,
};

const NOW: i64 = 1_800_000_000;
const AUDIENCE: &str = "https://verifier.example";
const NONCE: &str = "n-0S6_WzA2Mj";

fn policy() -> KeyBindingPolicy {
    KeyBindingPolicy {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        now: NOW,
        max_age: 300,
        leeway: 60,
    }
}

fn required() -> KeyBindingRequirement {
    KeyBindingRequirement::Required(policy())
}

fn binding() -> KeyBinding {
    KeyBinding {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        issued_at: NOW,
    }
}

fn name() -> ClaimPath {
    ClaimPath::claim("credentialSubject").key("name")
}

/// The Issuer's key, and the Holder's, which are not the same key.
fn keys() -> (KeyEntry, KeyEntry) {
    (
        generated(KeyAlgorithm::EcdsaP256),
        generated(KeyAlgorithm::Ed25519),
    )
}

/// A credential issued to `holder`, with `name` concealed.
fn issued(issuer: &KeyEntry, holder: &KeyEntry) -> SdJwt {
    sign_credential_sd_jwt(
        &credential(),
        issuer,
        KID,
        &[],
        &[name()],
        Some(&holder.public_key()),
    )
    .expect("signs")
}

/// Every Disclosure `sd_jwt` holds, chosen.
fn reveal_all(sd_jwt: &SdJwt) -> Vec<String> {
    sd_jwt.disclosures().iter().map(|d| d.digest()).collect()
}

/// `issued`, presented with every Disclosure and bound by `holder` to `binding`.
fn bound(sd_jwt: &SdJwt, holder: &KeyEntry, binding: &KeyBinding) -> String {
    let digests = reveal_all(sd_jwt);
    let reveal: Vec<&str> = digests.iter().map(String::as_str).collect();
    sd_jwt
        .present_with_key_binding(&reveal, binding, holder)
        .expect("binds")
        .serialise()
}

fn verify(
    token: &str,
    issuer: &KeyEntry,
    binding: &KeyBindingRequirement,
) -> Result<Value, JoseError> {
    verify_credential_sd_jwt(
        token,
        &issuer.public_key(),
        VerifyOptions::new().at(NOW),
        binding,
    )
}

/// What the Key Binding JWT was refused for, by the name of the check.
fn refused_for(result: Result<Value, JoseError>, check: &str) {
    assert_eq!(
        result,
        Err(JoseError::KeyBindingRefused {
            reason: format!("the Key Binding JWT is not valid: {check}")
        })
    );
}

// ── required, and satisfied ───────────────────────────────────────────────────

#[test]
fn a_bound_credential_verifies_when_binding_is_required_and_the_policy_matches() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);

    let verified =
        verify(&bound(&sd_jwt, &holder, &binding()), &issuer, &required()).expect("verifies");

    assert_eq!(verified["credentialSubject"]["name"], "Example");
    assert_eq!(
        verified["cnf"],
        holder_key_confirmation(&holder.public_key())
    );
}

#[test]
fn a_bound_presentation_verifies_when_binding_is_required_and_the_policy_matches() {
    let (issuer, holder) = keys();
    let sd_jwt = sign_presentation_sd_jwt(
        &presentation(vec![]),
        &issuer,
        KID,
        &[],
        &[],
        Some(&holder.public_key()),
    )
    .expect("signs");
    let token = bound(&sd_jwt, &holder, &binding());

    let verified = verify_presentation_sd_jwt(
        &token,
        &issuer.public_key(),
        VerifyOptions::new().at(NOW),
        &required(),
    )
    .expect("verifies");

    assert_eq!(verified.payload()["holder"], "https://holder.example");
}

/// Every algorithm the Holder's key may have, since the check is made under the key `cnf`
/// names.
#[test]
fn a_bound_credential_verifies_for_each_holder_key_algorithm() {
    let issuer = generated(KeyAlgorithm::EcdsaP256);
    for algorithm in crate::test_support::EVERY_ALGORITHM {
        let holder = generated(algorithm);
        let sd_jwt = issued(&issuer, &holder);

        assert!(
            verify(&bound(&sd_jwt, &holder, &binding()), &issuer, &required()).is_ok(),
            "{algorithm:?}"
        );
    }
}

// ── required, and refused ─────────────────────────────────────────────────────

#[test]
fn a_wrong_nonce_is_refused() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    let token = bound(
        &sd_jwt,
        &holder,
        &KeyBinding {
            nonce: "another".to_owned(),
            ..binding()
        },
    );

    refused_for(verify(&token, &issuer, &required()), "nonce");
}

#[test]
fn a_wrong_audience_is_refused() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    let token = bound(
        &sd_jwt,
        &holder,
        &KeyBinding {
            audience: "https://elsewhere.example".to_owned(),
            ..binding()
        },
    );

    refused_for(verify(&token, &issuer, &required()), "aud");
}

/// Step 5.e: the creation time must be "within an acceptable window", which is the policy's
/// `max_age` behind `now` and its `leeway` ahead of it.
#[test]
fn an_iat_outside_the_window_is_refused() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    for issued_at in [NOW - 301, NOW + 61] {
        let token = bound(
            &sd_jwt,
            &holder,
            &KeyBinding {
                issued_at,
                ..binding()
            },
        );

        refused_for(verify(&token, &issuer, &required()), "iat");
    }
    for issued_at in [NOW - 300, NOW + 60] {
        let token = bound(
            &sd_jwt,
            &holder,
            &KeyBinding {
                issued_at,
                ..binding()
            },
        );

        assert!(verify(&token, &issuer, &required()).is_ok(), "{issued_at}");
    }
}

/// Step 5.g: the `sd_hash` covers the Issuer-signed JWT and the Disclosures. A Key Binding JWT
/// made for a presentation that disclosed less does not bind one that discloses more.
#[test]
fn a_tampered_sd_hash_is_refused() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    let withheld = sd_jwt
        .present_with_key_binding(&[], &binding(), &holder)
        .expect("binds")
        .serialise();
    let kb = withheld.rsplit('~').next().expect("a Key Binding JWT");
    let disclosing_more = format!("{}{kb}", sd_jwt.serialise());

    refused_for(verify(&disclosing_more, &issuer, &required()), "sd_hash");
}

#[test]
fn a_key_binding_jwt_signed_by_another_key_is_refused() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    let intruder = generated(KeyAlgorithm::Ed25519);
    let digests = reveal_all(&sd_jwt);
    let reveal: Vec<&str> = digests.iter().map(String::as_str).collect();
    // The intruder cannot bind a presentation to a key it does not hold, so it signs a Key
    // Binding JWT for the right `sd_hash` and attaches it by hand.
    let presented = sd_jwt.present(&reveal).expect("presents");
    let kb = forge(
        &intruder,
        &json!({"alg": "EdDSA", "typ": "kb+jwt"}),
        &json!({
            "iat": NOW,
            "aud": AUDIENCE,
            "nonce": NONCE,
            "sd_hash": presented.sd_hash(),
        }),
    );
    let token = format!("{}{kb}", presented.serialise());

    refused_for(verify(&token, &issuer, &required()), "the signature");
}

/// Step 2: "If Key Binding is required and the Holder has provided an SD-JWT (without Key
/// Binding), the Verifier MUST reject the presentation."
#[test]
fn a_token_with_no_key_binding_jwt_is_refused_when_binding_is_required() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);

    assert_eq!(
        verify(&sd_jwt.serialise(), &issuer, &required()),
        Err(JoseError::KeyBindingMissing)
    );
}

/// Without a Holder key in the processed payload, there is nothing to check the Key Binding
/// JWT against, whatever it says.
#[test]
fn a_token_with_no_cnf_is_refused_when_binding_is_required() {
    let (issuer, holder) = keys();
    let sd_jwt =
        sign_credential_sd_jwt(&credential(), &issuer, KID, &[], &[name()], None).expect("signs");
    let digests = reveal_all(&sd_jwt);
    let reveal: Vec<&str> = digests.iter().map(String::as_str).collect();
    let presented = sd_jwt.present(&reveal).expect("presents");
    let kb = forge(
        &holder,
        &json!({"alg": "EdDSA", "typ": "kb+jwt"}),
        &json!({
            "iat": NOW,
            "aud": AUDIENCE,
            "nonce": NONCE,
            "sd_hash": presented.sd_hash(),
        }),
    );
    let token = format!("{}{kb}", presented.serialise());

    assert_eq!(
        verify(&token, &issuer, &required()),
        Err(JoseError::KeyBindingRefused {
            reason: "the token names no Holder key in cnf".to_owned()
        })
    );
}

/// The Issuer's signature is checked before the Key Binding JWT is read.
#[test]
fn the_issuer_signature_is_checked_before_the_key_binding() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    let token = bound(&sd_jwt, &holder, &binding());
    let other = generated(KeyAlgorithm::EcdsaP256);

    assert_eq!(
        verify(&token, &other, &required()),
        Err(JoseError::SignatureDoesNotVerify)
    );
}

// ── not required ──────────────────────────────────────────────────────────────

/// §7.3 steps 3 and 4: when Key Binding is not required, the token is processed as an SD-JWT,
/// and the Key Binding JWT is not read. So it verifies, whatever the Key Binding JWT says.
#[test]
fn a_bound_credential_is_accepted_when_binding_is_not_required() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    let wrong = KeyBinding {
        nonce: "another".to_owned(),
        audience: "https://elsewhere.example".to_owned(),
        issued_at: 1,
    };

    for binding in [binding(), wrong] {
        let token = bound(&sd_jwt, &holder, &binding);

        let verified = verify(&token, &issuer, &NOT_REQUIRED).expect("verifies");

        assert_eq!(verified["credentialSubject"]["name"], "Example");
    }
}

/// §9.5: the choice is not the token's. The bare token and the bound one meet the same
/// choice, and the same token is refused or accepted by the choice alone.
#[test]
fn the_choice_is_the_callers_and_not_the_tokens() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    let bare = sd_jwt.serialise();
    let with_kb = bound(&sd_jwt, &holder, &binding());

    assert!(verify(&bare, &issuer, &NOT_REQUIRED).is_ok());
    assert!(verify(&with_kb, &issuer, &NOT_REQUIRED).is_ok());
    assert!(verify(&bare, &issuer, &required()).is_err());
    assert!(verify(&with_kb, &issuer, &required()).is_ok());
}

// ── envelopes ─────────────────────────────────────────────────────────────────

/// The binding decision reaches the credentials in a presentation's envelopes. A `vc+jwt`
/// envelope cannot carry a Key Binding JWT, so where binding is required it is refused, as
/// an SD-JWT presented without one is (RFC 9901 §7.3 step 2), and where it is not, it is
/// verified as before.
#[test]
fn the_envelope_path_takes_the_decision_too() {
    let (issuer, holder) = keys();
    let sd_jwt = issued(&issuer, &holder);
    let bound_token = bound(&sd_jwt, &holder, &binding());
    let bare_token = sd_jwt.serialise();
    let jwt_token = sign_credential(&credential(), &issuer, KID, &[]).expect("signs");
    let outer = |envelopes: Vec<Value>| {
        let token = sign_presentation(&presentation(envelopes), &holder, KID, &[]).expect("signs");
        verify_presentation(&token, &holder.public_key(), VerifyOptions::new()).expect("verifies")
    };
    let options = VerifyOptions::new().at(NOW);
    let issuer_public = issuer.public_key();

    let bound_in = outer(vec![sd_envelope(&bound_token)]);
    assert!(
        bound_in
            .verify_credentials(options, &required(), |_| Some(issuer_public.clone()))
            .is_ok()
    );
    assert!(
        bound_in.envelopes()[0]
            .verify(&issuer_public, options, &required())
            .is_ok()
    );

    let bare_in = outer(vec![sd_envelope(&bare_token)]);
    assert_eq!(
        bare_in.verify_credentials(options, &required(), |_| Some(issuer_public.clone())),
        Err(JoseError::CredentialRefused {
            index: 0,
            source: Box::new(JoseError::KeyBindingMissing)
        })
    );
    assert!(
        bare_in
            .verify_credentials(options, &NOT_REQUIRED, |_| Some(issuer_public.clone()))
            .is_ok()
    );

    let jwt_in = outer(vec![envelope(&jwt_token)]);
    assert_eq!(
        jwt_in.verify_credentials(options, &required(), |_| Some(issuer_public.clone())),
        Err(JoseError::CredentialRefused {
            index: 0,
            source: Box::new(JoseError::KeyBindingMissing)
        })
    );
    assert!(
        jwt_in
            .verify_credentials(options, &NOT_REQUIRED, |_| Some(issuer_public.clone()))
            .is_ok()
    );
}
