//! Key Binding for SD-JWT VCs: a Holder key named at issuance, and a policy at
//! verification.

use serde_json::{Map, Value, json};

use dpp_crypto::keystore::{KeyEntry, KeyStore};
use dpp_crypto::sd_jwt::{
    KeyBinding, KeyBindingPolicy, SdJwt, SdJwtError, build_payload, conceal,
    holder_key_confirmation,
};
use dpp_domain::ProductGroup;

use super::test_fixtures::*;
use super::{Issuance, PASSPORT_CLAIM, SdJwtVcError, TYP, issue, vct_for, verify};
use crate::test_support::temp_store;

const AUDIENCE: &str = "https://verifier.example";
const NONCE: &str = "n-0S6_WzA2Mj";

fn vct() -> String {
    vct_for(ProductGroup::Battery, SCHEMA_VERSION)
}

fn holder() -> KeyEntry {
    temp_store("sd-jwt-vc-holder", "holder")
        .load_key("holder")
        .expect("the key was just generated")
}

/// The fixture credential, bound to `holder`.
fn bound(store: &KeyStore, holder: &KeyEntry) -> SdJwt {
    issue(
        store,
        KEY_ID,
        &payload(),
        &policy(),
        &Issuance {
            issuer: ISSUER,
            vct: &vct(),
            issued_at: ISSUED_AT,
            holder_key: Some(holder.verifying_key.to_bytes()),
        },
    )
    .expect("issue")
}

/// What the Holder binds to, a minute after issuance.
fn binding(nonce: &str) -> KeyBinding {
    KeyBinding {
        audience: AUDIENCE.to_owned(),
        nonce: nonce.to_owned(),
        issued_at: ISSUED_AT + 60,
    }
}

/// The Verifier's policy, at the instant [`now`] names.
fn key_binding_policy() -> KeyBindingPolicy {
    KeyBindingPolicy {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        now: ISSUED_AT + 60,
        max_age: 300,
        leeway: 5,
    }
}

fn verify_as(
    token: &str,
    store: &KeyStore,
    policy: Option<&KeyBindingPolicy>,
) -> Result<Map<String, Value>, SdJwtVcError> {
    verify(token, &public_key_b64(store), &vct(), now(), policy)
}

/// The JWT's payload as signed, without processing any Disclosure.
fn signed_payload(credential: &SdJwt) -> Value {
    use base64::Engine;
    let part = credential.jwt().split('.').nth(1).unwrap();
    serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(part)
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn a_bound_credential_names_the_holder_key_in_the_clear() {
    let store = temp_store("sd-jwt-vc-kb", KEY_ID);
    let holder = holder();
    let credential = bound(&store, &holder);

    assert_eq!(
        signed_payload(&credential)["cnf"],
        holder_key_confirmation(&holder.verifying_key.to_bytes())
    );
    assert!(
        credential
            .disclosures()
            .iter()
            .all(|d| d.claim_name() != Some("cnf")),
        "clause 2.2.2.3: cnf MUST NOT be in a Disclosure"
    );

    let unbound = issued(&store);
    assert!(signed_payload(&unbound).get("cnf").is_none());
}

#[test]
fn a_bound_presentation_verifies_under_the_policy_and_reads_to_what_was_chosen() {
    let store = temp_store("sd-jwt-vc-kb", KEY_ID);
    let holder = holder();
    let credential = bound(&store, &holder);
    let reveal = digests(&credential, &["cathodeMaterial"]);
    let reveal: Vec<&str> = reveal.iter().map(String::as_str).collect();
    let presentation = credential
        .present_with_key_binding(&reveal, &binding(NONCE), &holder)
        .expect("the Holder binds its presentation")
        .serialise();

    let claims = verify_as(&presentation, &store, Some(&key_binding_policy()))
        .expect("a bound presentation verifies");
    assert_eq!(
        claims[PASSPORT_CLAIM]["productGroupData"]["cathodeMaterial"],
        json!(["LiFePO4"])
    );
    assert!(
        claims[PASSPORT_CLAIM]["productGroupData"]
            .get("stateOfHealthPct")
            .is_none()
    );
    assert_eq!(
        claims["cnf"],
        holder_key_confirmation(&holder.verifying_key.to_bytes())
    );
}

/// RFC 9901 clause 4: a Verifier that expects an SD-JWT MUST find the last
/// segment empty, and `None` says this one expects no Key Binding.
#[test]
fn a_reader_that_asks_for_no_key_binding_refuses_a_token_that_carries_one() {
    let store = temp_store("sd-jwt-vc-kb", KEY_ID);
    let holder = holder();
    let presentation = bound(&store, &holder)
        .present_with_key_binding(&[], &binding(NONCE), &holder)
        .unwrap()
        .serialise();

    assert_eq!(
        verify_as(&presentation, &store, None),
        Err(SdJwtVcError::UnexpectedKeyBinding)
    );
}

/// Clause 7.3 step 1: the policy decides, never the token's shape. A copy of a
/// bound credential presented without a Key Binding JWT is what Key Binding
/// exists to refuse.
#[test]
fn a_policy_refuses_a_bound_credential_presented_without_key_binding() {
    let store = temp_store("sd-jwt-vc-kb", KEY_ID);
    let holder = holder();
    let copy = bound(&store, &holder).serialise();

    assert_eq!(
        verify_as(&copy, &store, Some(&key_binding_policy())),
        Err(SdJwtVcError::SdJwt(SdJwtError::KeyBindingMissing))
    );
    assert!(
        verify_as(&copy, &store, None).is_ok(),
        "control: the same token verifies when Key Binding is not asked for"
    );
}

#[test]
fn a_policy_refuses_a_credential_that_names_no_holder_key() {
    let store = temp_store("sd-jwt-vc-kb", KEY_ID);
    let holder = holder();
    // A valid Key Binding JWT, from a bound presentation, attached to a credential
    // that names no key: there is nothing to check its signature against.
    let bound_token = bound(&store, &holder)
        .present_with_key_binding(&[], &binding(NONCE), &holder)
        .unwrap()
        .serialise();
    let kb_jwt = bound_token.rsplit('~').next().unwrap();
    let unbound = format!(
        "{}{kb_jwt}",
        issued(&store).present(&[]).unwrap().serialise()
    );

    assert_eq!(
        verify_as(&unbound, &store, Some(&key_binding_policy())),
        Err(SdJwtVcError::SdJwt(SdJwtError::NoHolderKey))
    );
}

#[test]
fn a_key_binding_jwt_for_another_transaction_is_refused() {
    let store = temp_store("sd-jwt-vc-kb", KEY_ID);
    let holder = holder();
    let replayed = bound(&store, &holder)
        .present_with_key_binding(&[], &binding("an earlier nonce"), &holder)
        .unwrap()
        .serialise();

    assert_eq!(
        verify_as(&replayed, &store, Some(&key_binding_policy())),
        Err(SdJwtVcError::SdJwt(SdJwtError::KeyBindingRejected("nonce")))
    );
}

/// A credential signed by the fixture key, with the registered claims clause
/// 2.2.2.3 lists, each with a valid value, and the members `conceal_if` selects
/// moved into Disclosures. `cnf.jwk` is concealed inside `cnf` when `nested` is
/// set, which leaves `cnf` itself in the clear.
fn credential_concealing(store: &KeyStore, conceal_if: &[&str], nested: bool) -> SdJwt {
    let holder = holder();
    let mut root: Map<String, Value> = serde_json::from_value(json!({
        "iss": ISSUER,
        "vct": vct(),
        "iat": ISSUED_AT,
        "nbf": ISSUED_AT - 10,
        "exp": ISSUED_AT + 3600,
        "cnf": holder_key_confirmation(&holder.verifying_key.to_bytes()),
        "vct#integrity": "sha256-WUbW3HSBb8BUkkn7WN2xKkGXRx6gDwYn8vFLnzzYfMM",
        "aka_vcts": ["https://credentials.example/other"],
        "status": { "status_list": { "idx": 0, "uri": "https://status.example/1" } },
        "name": "battery",
    }))
    .unwrap();

    let mut disclosures = Vec::new();
    if nested {
        let cnf = root["cnf"].as_object().unwrap().clone();
        let (kept, mut inner) = conceal(&cnf, |name| name == "jwk").unwrap();
        root.insert("cnf".to_owned(), Value::Object(kept));
        disclosures.append(&mut inner);
    }
    let (kept, mut outer) = conceal(&root, |name| conceal_if.contains(&name)).unwrap();
    disclosures.append(&mut outer);

    let payload = build_payload(kept, !disclosures.is_empty());
    let jwt =
        dpp_crypto::jws::sign_typed(store, KEY_ID, &payload, "kid", Some(TYP), None).expect("sign");
    SdJwt::new(jwt, disclosures)
}

/// Draft-19 clause 2.2.2.3: these claims, and their sub-claims, MUST NOT be
/// included in Disclosures. RFC 9901 clause 9.7 only advises an Issuer against
/// it, so the profile is the stricter of the two, and `verify` holds a credential
/// to it.
#[test]
fn each_claim_the_profile_keeps_out_of_disclosures_is_refused_when_disclosed() {
    let store = temp_store("sd-jwt-vc-kb", KEY_ID);
    assert!(
        verify_as(
            &credential_concealing(&store, &["name"], false).serialise(),
            &store,
            None
        )
        .is_ok(),
        "control: every listed claim in the clear, and another concealed"
    );

    for claim in [
        "iss",
        "nbf",
        "exp",
        "cnf",
        "vct",
        "vct#integrity",
        "aka_vcts",
        "status",
    ] {
        let credential = credential_concealing(&store, &[claim], false);
        assert_eq!(
            verify_as(&credential.serialise(), &store, None),
            Err(SdJwtVcError::DisclosedClaim(claim)),
            "{claim} arrived in a Disclosure"
        );
    }

    let nested = credential_concealing(&store, &[], true);
    assert_eq!(
        verify_as(&nested.serialise(), &store, None),
        Err(SdJwtVcError::DisclosedClaim("cnf")),
        "a sub-claim of cnf arrived in a Disclosure"
    );
}

/// The limit of the check, pinned so it is not mistaken for more: a Disclosure the
/// Holder withholds is not in the presentation, so the Verifier cannot tell that
/// the claim was ever concealed. Keeping these claims in the clear is the
/// Issuer's half of clause 2.2.2.3.
#[test]
fn a_withheld_disclosure_of_such_a_claim_cannot_be_seen() {
    let store = temp_store("sd-jwt-vc-kb", KEY_ID);
    let withheld = credential_concealing(&store, &["exp"], false)
        .present(&[])
        .unwrap()
        .serialise();

    let claims = verify_as(&withheld, &store, None).expect("nothing disclosed, nothing to refuse");
    assert!(claims.get("exp").is_none());
}
