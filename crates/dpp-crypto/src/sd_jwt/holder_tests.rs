//! The Holder's duties of RFC 9901 clause 7.2 that [`SdJwt::present`] can see.
//!
//! Two of them. An SD-JWT+KB is not something an Issuer hands over, so it is not
//! presented again. And each Disclosure chosen must be referenced by the
//! Issuer-signed JWT or by another one chosen (step 2), which is the recursive
//! structure of clause 4.2.6 seen from the Holder's side: the RFC's own example
//! says a nationality cannot travel without the Disclosure of `nationalities`.

use serde_json::{Map, Value, json};

use super::tests::stub_jwt;
use super::{Disclosure, SdJwt, SdJwtError, build_payload, conceal, conceal_elements};

/// Clause 4.2.6's example: every nationality hidden, then the array itself.
fn nationalities() -> (SdJwt, Vec<Disclosure>, Disclosure) {
    let (placeholders, elements) =
        conceal_elements(&[json!("DE"), json!("FR"), json!("UK")], |_, _| true);
    let mut claims = Map::new();
    claims.insert("family_name".to_owned(), json!("Möbius"));
    claims.insert("nationalities".to_owned(), Value::Array(placeholders));
    let (kept, mut outer) = conceal(&claims, |name| name == "nationalities").unwrap();
    let array = outer.pop().expect("the array was concealed");
    let mut disclosures = elements.clone();
    disclosures.push(array.clone());
    let sd_jwt = SdJwt::new(stub_jwt(&build_payload(kept, true)), disclosures);
    (sd_jwt, elements, array)
}

#[test]
fn an_element_chosen_without_the_array_that_holds_it_is_not_presented() {
    let (sd_jwt, elements, _) = nationalities();
    let de = elements[0].digest();
    assert_eq!(
        sd_jwt.present(&[&de]).map(|p| p.disclosures().len()),
        Err(SdJwtError::UnusedDisclosures(1)),
        "a Verifier has nowhere to put DE without the Disclosure of nationalities"
    );
}

#[test]
fn an_element_chosen_with_the_array_that_holds_it_is() {
    let (sd_jwt, elements, array) = nationalities();
    let (de, outer) = (elements[0].digest(), array.digest());
    let presented = sd_jwt.present(&[&de, &outer]).expect("both are referenced");
    assert_eq!(presented.disclosures().len(), 2);
    let payload = presented.disclosed_payload().expect("processes");
    assert_eq!(
        Value::Object(payload),
        json!({ "family_name": "Möbius", "nationalities": ["DE"] })
    );
}

#[test]
fn a_member_chosen_without_the_object_that_holds_it_is_not_presented() {
    let address = json!({ "locality": "Musterstadt", "country": "DE" });
    let (inner, mut members) =
        conceal(address.as_object().unwrap(), |name| name == "locality").unwrap();
    let mut claims = Map::new();
    claims.insert("address".to_owned(), Value::Object(inner));
    let (kept, mut outer) = conceal(&claims, |name| name == "address").unwrap();
    let locality = members.pop().unwrap();
    let address = outer.pop().unwrap();
    let sd_jwt = SdJwt::new(
        stub_jwt(&build_payload(kept, true)),
        vec![locality.clone(), address.clone()],
    );

    assert_eq!(
        sd_jwt.present(&[&locality.digest()]).err(),
        Some(SdJwtError::UnusedDisclosures(1))
    );
    assert!(sd_jwt.present(&[&address.digest()]).is_ok());
    assert!(
        sd_jwt
            .present(&[&address.digest(), &locality.digest()])
            .is_ok()
    );
}

#[test]
fn a_token_that_carries_a_key_binding_jwt_is_not_presented() {
    let (sd_jwt, _, array) = nationalities();
    let kb = stub_jwt(&json!({ "iat": 1, "aud": "v", "nonce": "n", "sd_hash": "x" }));
    let received = SdJwt::parse(&format!("{}{kb}", sd_jwt.serialise())).unwrap();
    assert!(received.has_key_binding());
    for chosen in [vec![], vec![array.digest()]] {
        let chosen: Vec<&str> = chosen.iter().map(String::as_str).collect();
        assert_eq!(
            received.present(&chosen).err(),
            Some(SdJwtError::KeyBindingAttached),
            "an SD-JWT+KB is refused whatever is chosen from it"
        );
    }
}

#[test]
fn presenting_nothing_sends_the_signed_jwt_alone() {
    let (sd_jwt, _, _) = nationalities();
    let presented = sd_jwt
        .present(&[])
        .expect("nothing chosen is a valid choice");
    assert_eq!(presented.serialise(), format!("{}~", sd_jwt.jwt()));
    assert_eq!(
        Value::Object(presented.disclosed_payload().unwrap()),
        json!({ "family_name": "Möbius" })
    );
}
