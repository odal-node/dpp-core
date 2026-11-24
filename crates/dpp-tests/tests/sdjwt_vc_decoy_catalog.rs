//! A product group's credential has one shape, whatever the passport holds.
//!
//! An issuer that pads each object to what its schema allows hides how many restricted
//! members a passport carries, and whether it carries a given one at all. That is a claim
//! about every product group at once, and a schema that gains a restricted member changes
//! it, so it is held over the whole catalog: for each group, at its current version, a
//! passport that carries every restricted member the policy names and one that carries
//! none issue credentials whose `_sd` arrays are the same, object for object.
//!
//! Where a group's passport is also built from the envelope's restricted fields, they are
//! present in one and absent in the other for the same reason.

use std::collections::BTreeMap;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use dpp_crypto::keystore::KeyStore;
use dpp_crypto::sd_jwt::SdJwt;
use dpp_domain::access::{DocumentScope, ProductGroupAccessPolicy};
use dpp_domain::{ProductGroup, ProductGroupCatalog};
use dpp_tests::fixtures::{
    passport_with, schema_compat_fixture, schema_for, with_restricted_envelope_members,
    with_restricted_members,
};
use dpp_vc::sd_jwt_vc::{Issuance, PASSPORT_CLAIM, issue, vct_for};
use serde_json::Value;

const KEY_ID: &str = "issuer";
const ISSUED_AT: i64 = 1_789_000_000;
const PRODUCT_GROUP_DATA: &str = "productGroupData";

/// For each object that carries an `_sd` array, its path from the credential's root, with
/// array indices left out as the policy leaves them, and the length of that array. Objects
/// of one array share a path, so a path holds a list.
fn sd_shape(payload: &Value) -> BTreeMap<String, Vec<usize>> {
    fn walk(value: &Value, path: &str, out: &mut BTreeMap<String, Vec<usize>>) {
        match value {
            Value::Object(map) => {
                if let Some(Value::Array(sd)) = map.get("_sd") {
                    out.entry(path.to_owned()).or_default().push(sd.len());
                }
                for (key, inner) in map {
                    if key != "_sd" {
                        walk(inner, &format!("{path}/{key}"), out);
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, path, out);
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeMap::new();
    walk(payload, "", &mut out);
    for lengths in out.values_mut() {
        lengths.sort_unstable();
    }
    out
}

struct Issued {
    credential: SdJwt,
    payload: Value,
    passport: Value,
}

/// The credential issued for a group's passport, carrying every restricted member its
/// policy names, or none.
fn issue_for(store: &KeyStore, key: &str, version: &str, carry: bool) -> Issued {
    let group = ProductGroup::from_wire_tag(key);
    let policy = ProductGroupAccessPolicy::for_passport(key, version)
        .unwrap_or_else(|| panic!("no access policy for {key} {version}"));
    let data = with_restricted_members(
        schema_compat_fixture(key, version),
        &schema_for(key, version),
        &policy,
        carry,
    );
    let mut passport = passport_with(&group, version, data);
    with_restricted_envelope_members(&mut passport, &policy, carry);

    let vct = vct_for(group, version);
    let credential = issue(
        store,
        KEY_ID,
        &passport,
        &policy,
        &Issuance {
            issuer: "https://passports.operator.example",
            vct: &vct,
            issued_at: ISSUED_AT,
            expires_at: Some(ISSUED_AT + 31_536_000),
            not_before: None,
            holder_key: None,
            status: None,
        },
    )
    .unwrap_or_else(|e| panic!("issuing {key} {version}: {e:?}"));

    let signed = credential
        .jwt()
        .split('.')
        .nth(1)
        .expect("a payload segment");
    let payload = serde_json::from_slice(&B64.decode(signed).expect("base64url")).expect("JSON");
    Issued {
        credential,
        payload,
        passport,
    }
}

fn store() -> (KeyStore, std::path::PathBuf) {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "test-sdjwt-vc-decoy-catalog-{}.json",
        uuid::Uuid::now_v7()
    ));
    let store =
        KeyStore::open(&path, &uuid::Uuid::now_v7().to_string()).expect("open the keystore");
    store.generate_key(KEY_ID).expect("generate the key");
    (store, path)
}

fn versions() -> Vec<(String, String)> {
    ProductGroupCatalog::new()
        .all()
        .iter()
        .map(|d| (d.key.clone(), d.current_schema_version.clone()))
        .collect()
}

#[test]
fn a_credential_has_one_shape_whatever_the_passport_holds() {
    let (store, path) = store();
    for (key, version) in versions() {
        let full = issue_for(&store, &key, &version, true);
        let empty = issue_for(&store, &key, &version, false);

        assert_eq!(
            sd_shape(&full.payload),
            sd_shape(&empty.payload),
            "{key} {version}: the `_sd` arrays depend on what the passport carries"
        );
        assert!(
            sd_shape(&empty.payload).contains_key(&format!("/{PASSPORT_CLAIM}")),
            "{key} {version}: the passport root carries no `_sd`, even of decoys"
        );
    }
    let _ = std::fs::remove_file(path);
}

/// Each array is as long as the policy says the object can hold. With every restricted
/// member present nothing is padded, so this is also the real count: it fails if the
/// issuer stops asking the policy, or asks about the wrong position.
#[test]
fn each_array_is_as_long_as_the_policy_says_its_object_can_hold() {
    let (store, path) = store();
    for (key, version) in versions() {
        let policy = ProductGroupAccessPolicy::for_passport(&key, &version).expect("a policy");
        let full = issue_for(&store, &key, &version, true);

        for (at, lengths) in sd_shape(&full.payload) {
            let segments: Vec<&str> = at.split('/').skip(1).collect();
            let (position, scope) = match segments.as_slice() {
                [claim] if *claim == PASSPORT_CLAIM => (Vec::new(), DocumentScope::Envelope),
                [claim, rest @ ..] if *claim == PASSPORT_CLAIM => match rest.split_first() {
                    Some((first, below)) if *first == PRODUCT_GROUP_DATA => {
                        (below.to_vec(), DocumentScope::ProductGroupData)
                    }
                    _ => (rest.to_vec(), DocumentScope::Envelope),
                },
                _ => continue,
            };
            let counted = policy.concealable_members(&position, scope);
            if counted == 0 {
                continue;
            }
            assert!(
                lengths.iter().all(|length| *length == counted),
                "{key} {version} at {at}: {lengths:?}, the policy says {counted}"
            );
        }
    }
    let _ = std::fs::remove_file(path);
}

/// The test above would pass if no group ever had an object below the root. One does, and
/// it is the elements of an array: battery's critical raw materials, each padded for the
/// members its schema lets it hold.
#[test]
fn some_group_carries_an_sd_array_below_the_root() {
    let (store, path) = store();
    let below = format!("/{PASSPORT_CLAIM}/{PRODUCT_GROUP_DATA}/");
    let (_, version) = versions()
        .into_iter()
        .find(|(key, _)| key == "battery")
        .expect("battery is in the catalog");
    for carry in [true, false] {
        let battery = issue_for(&store, "battery", &version, carry);
        let nested = sd_shape(&battery.payload)
            .keys()
            .filter(|at| at.starts_with(&below))
            .count();
        assert!(
            nested > 0,
            "battery has no `_sd` array below productGroupData"
        );

        let materials = battery.payload[PASSPORT_CLAIM][PRODUCT_GROUP_DATA]["criticalRawMaterials"]
            .as_array()
            .unwrap_or_else(|| panic!("criticalRawMaterials is not an array (carry {carry})"));
        assert!(
            !materials.is_empty()
                && materials
                    .iter()
                    .all(|element| element.get("_sd").is_some_and(Value::is_array)),
            "an element of criticalRawMaterials carries no `_sd` (carry {carry})"
        );
    }
    let _ = std::fs::remove_file(path);
}

/// Decoys read back as nothing: every credential gives back the passport it was made from.
#[test]
fn a_credential_padded_to_its_schema_reads_back_to_the_passport() {
    let (store, path) = store();
    for (key, version) in versions() {
        for carry in [true, false] {
            let issued = issue_for(&store, &key, &version, carry);
            let read = issued
                .credential
                .disclosed_payload()
                .unwrap_or_else(|e| panic!("{key} {version} (carry {carry}): {e:?}"));
            assert!(
                read.get(PASSPORT_CLAIM)
                    .is_some_and(|read| same(read, &issued.passport)),
                "{key} {version} (carry {carry})"
            );
        }
    }
    let _ = std::fs::remove_file(path);
}

/// Equal as JSON values, with a number equal to another of the same value: `12` and
/// `12.0` are one number, and the token does not keep which spelling went in.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| same(a, b)))
        }
        _ => a == b,
    }
}
