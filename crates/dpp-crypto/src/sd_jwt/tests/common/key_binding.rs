//! A Holder's key, a binding and the presentations the Key Binding tests build.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde_json::{Map, Value, json};

use super::super::super::{
    KeyBinding, KeyBindingPolicy, SdJwt, build_payload, conceal, holder_key_confirmation,
};
use super::stub_jwt;
use crate::jws::signer::sign_with_header;
use crate::keystore::KeyEntry;
use crate::test_support::temp_store;

pub(crate) fn holder(label: &str) -> KeyEntry {
    temp_store(label, "holder")
        .load_key("holder")
        .expect("the key was just made")
}

pub(crate) fn binding() -> KeyBinding {
    KeyBinding {
        audience: "https://verifier.example".to_owned(),
        nonce: "n-0S6_WzA2Mj".to_owned(),
        issued_at: NOW,
    }
}

pub(crate) fn policy() -> KeyBindingPolicy {
    KeyBindingPolicy {
        audience: "https://verifier.example".to_owned(),
        nonce: "n-0S6_WzA2Mj".to_owned(),
        now: NOW,
        max_age: 300,
        leeway: 60,
    }
}

/// A token whose Issuer named `cnf`, with `given_name` and `family_name` hidden.
pub(crate) fn issued_to(cnf: Value) -> SdJwt {
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

pub(crate) fn issued(holder: &KeyEntry) -> SdJwt {
    issued_to(holder_key_confirmation(&holder.public_key()))
}

pub(crate) fn every_digest(sd_jwt: &SdJwt) -> Vec<String> {
    sd_jwt.disclosures().iter().map(|d| d.digest()).collect()
}

pub(crate) fn decoded(jwt: &str, part: usize) -> Map<String, Value> {
    let bytes = B64.decode(jwt.split('.').nth(part).unwrap()).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// `presentation` with its Key Binding JWT replaced by one signed by `key` over
/// `header` and `claims`, so a test can break one thing at a time.
pub(crate) fn rebound(presentation: &SdJwt, key: &KeyEntry, header: Value, claims: Value) -> SdJwt {
    let header = header.as_object().unwrap().clone();
    let kb = sign_with_header(key, &header, claims.to_string().as_bytes())
        .expect("an unrevoked key signs");
    let unbound = presentation.serialise();
    let (sd_jwt, _) = unbound.rsplit_once('~').unwrap();
    SdJwt::parse(&format!("{sd_jwt}~{kb}")).unwrap()
}

pub(crate) fn good_claims(presentation: &SdJwt) -> Value {
    json!({
        "iat": NOW,
        "aud": "https://verifier.example",
        "nonce": "n-0S6_WzA2Mj",
        "sd_hash": presentation.sd_hash(),
    })
}

pub(crate) const NOW: i64 = 1_800_000_000;
