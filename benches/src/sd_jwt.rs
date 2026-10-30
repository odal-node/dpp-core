//! SD-JWT (RFC 9901) and the SD-JWT VC profile built on it: concealing claims,
//! the wire format, and issuing and verifying a whole passport as a credential.
//!
//! Concealment draws a fresh salt per claim, so the cost is the hash and the
//! random draw together. Issuance walks the passport with its access policy and
//! then signs, so it is the heaviest single operation in this target.

use base64::Engine;
use chrono::{TimeZone, Utc};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dpp_crypto::sd_jwt::{SdJwt, conceal, conceal_elements};
use dpp_domain::access::ProductGroupAccessPolicy;
use dpp_domain::{Passport, ProductGroup};
use dpp_vc::sd_jwt_vc::{Issuance, issue, vct_for, verify};
use serde_json::{Map, Value, json};

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::{battery_passport, keystore};

const KEY_ID: &str = "bench";
const ISSUER: &str = "https://passports.operator.example";
const ISSUED_AT: i64 = 1_789_000_000;

/// A JSON object of `members` claims.
fn object_of(members: usize) -> Map<String, Value> {
    (0..members)
        .map(|i| (format!("claim-{i:03}"), json!({"value": i, "unit": "kg"})))
        .collect()
}

/// Concealing the members of an object and the elements of an array.
fn concealment(c: &mut Criterion) {
    let mut objects = c.benchmark_group("sd_jwt_conceal_object");
    for members in [10_usize, 100] {
        let object = object_of(members);
        objects.bench_with_input(
            BenchmarkId::from_parameter(members),
            &object,
            |b, object| {
                b.iter(|| conceal(object, |_| true).unwrap());
            },
        );
    }
    objects.finish();

    let elements: Vec<Value> = (0..100).map(|i| json!({"part": i})).collect();
    c.bench_function("sd_jwt_conceal_elements_100", |b| {
        b.iter(|| conceal_elements(&elements, |_, _| true));
    });
}

/// Issuing and verifying a battery passport as an SD-JWT VC, and the wire format:
/// serialising, parsing, and disclosing all or half of the claims.
fn credential(c: &mut Criterion) {
    let (store, _dir) = keystore(KEY_ID);
    let passport: Passport = battery_passport();
    let payload = serde_json::to_value(&passport).unwrap();
    let policy = ProductGroupAccessPolicy::for_passport("battery", &passport.schema_version)
        .expect("the battery schema version is embedded");
    let vct = vct_for(ProductGroup::Battery, &passport.schema_version);
    let issuance = Issuance {
        issuer: ISSUER,
        vct: &vct,
        issued_at: ISSUED_AT,
        holder_key: None,
    };

    c.bench_function("sd_jwt_vc_issue_battery", |b| {
        b.iter(|| issue(&store, KEY_ID, &payload, &policy, &issuance).unwrap());
    });

    let issued = issue(&store, KEY_ID, &payload, &policy, &issuance).unwrap();
    assert!(
        issued.disclosures().len() > 1,
        "the fixture should conceal several claims, or present/disclose have nothing to do"
    );
    let serialised = issued.serialise();
    let now = Utc.timestamp_opt(ISSUED_AT + 60, 0).unwrap();
    let public_key = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(store.load_key(KEY_ID).unwrap().verifying_key.as_bytes());

    c.bench_function("sd_jwt_vc_verify_battery", |b| {
        b.iter(|| verify(&serialised, &public_key, &vct, now, None).unwrap());
    });

    // The wire format on its own, without the signature check.
    c.bench_function("sd_jwt_serialise", |b| b.iter(|| issued.serialise()));
    c.bench_function("sd_jwt_parse", |b| {
        b.iter(|| SdJwt::parse(&serialised).unwrap());
    });
    c.bench_function("sd_jwt_disclosed_payload_all", |b| {
        b.iter(|| issued.disclosed_payload().unwrap());
    });

    // A holder revealing half of what was concealed.
    let digests: Vec<String> = issued
        .disclosures()
        .iter()
        .step_by(2)
        .map(|d| d.digest())
        .collect();
    let reveal: Vec<&str> = digests.iter().map(String::as_str).collect();
    c.bench_function("sd_jwt_present_half", |b| {
        b.iter(|| issued.present(&reveal).unwrap());
    });
}

criterion_group!(benches, concealment, credential);
criterion_main!(benches);
