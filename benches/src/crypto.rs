use base64::Engine;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dpp_crypto::{
    jws::canonicalize, jws::signer, jws::verifier as jws_verifier, keystore::KeyStore,
};
use serde_json::json;

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::battery_passport_with_materials;

fn setup() -> (KeyStore, String, serde_json::Value, String, String) {
    let path = std::env::temp_dir().join(format!("bench-crypto-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "bench-pass").expect("open store");
    store.generate_key("bench").expect("generate key");

    let payload = json!({"passport_id": "bench-001", "status": "draft"});
    let jws = signer::sign(&store, "bench", &payload, "bench-kid").expect("sign");

    let entry = store.load_key("bench").expect("load key");
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let pub_key_b64 = b64.encode(entry.verifying_key.as_bytes());

    (store, "bench".to_owned(), payload, jws, pub_key_b64)
}

fn crypto_benchmarks(c: &mut Criterion) {
    let (store, key_id, payload, jws, pub_key_b64) = setup();

    c.bench_function("jws_sign", |b| {
        b.iter(|| signer::sign(&store, &key_id, &payload, "bench-kid").unwrap());
    });

    c.bench_function("jws_verify_via_store", |b| {
        b.iter(|| signer::verify(&store, &key_id, &jws).unwrap());
    });

    c.bench_function("jws_verify_standalone", |b| {
        b.iter(|| jws_verifier::verify_jws(&jws, &pub_key_b64).unwrap());
    });

    // The payload above is two keys; a published passport is the whole record.
    // Signing canonicalises the payload and then signs it, so the cost grows with
    // the document, and Ed25519 itself is the constant part. Throughput is the
    // canonical size of what is signed.
    let signed: Vec<_> = [1_usize, 50, 500]
        .into_iter()
        .map(|materials| {
            let payload = serde_json::to_value(battery_passport_with_materials(materials)).unwrap();
            let bytes = canonicalize(&payload).unwrap().len() as u64;
            let jws = signer::sign(&store, &key_id, &payload, "bench-kid").unwrap();
            (materials, payload, jws, bytes)
        })
        .collect();

    let mut sign = c.benchmark_group("jws_sign_passport");
    for (materials, payload, _, bytes) in &signed {
        sign.throughput(Throughput::Bytes(*bytes));
        sign.bench_with_input(
            BenchmarkId::from_parameter(materials),
            payload,
            |b, payload| b.iter(|| signer::sign(&store, &key_id, payload, "bench-kid").unwrap()),
        );
    }
    sign.finish();

    let mut verify = c.benchmark_group("jws_verify_passport");
    for (materials, _, jws, bytes) in &signed {
        verify.throughput(Throughput::Bytes(*bytes));
        verify.bench_with_input(BenchmarkId::from_parameter(materials), jws, |b, jws| {
            b.iter(|| jws_verifier::verify_jws(jws, &pub_key_b64).unwrap())
        });
    }
    verify.finish();
}

/// Opening a store derives its keys with Argon2id, which is slow on purpose: the
/// cost is the work factor an attacker with a copy of the file also pays per
/// guess. It is paid once per process start, not per request. A faster figure
/// here is a weaker store, not an optimisation, so this is here to make a change
/// to the parameters visible, not to be driven down.
fn keystore_benchmarks(c: &mut Criterion) {
    let path = std::env::temp_dir().join(format!("bench-open-{}.json", uuid::Uuid::now_v7()));
    KeyStore::open(&path, "bench-pass")
        .expect("create store")
        .generate_key("bench")
        .expect("generate key");

    let mut group = c.benchmark_group("keystore");
    group.sample_size(10);
    group.bench_function("open_existing", |b| {
        b.iter(|| KeyStore::open(&path, "bench-pass").unwrap());
    });
    group.finish();
}

criterion_group!(benches, crypto_benchmarks, keystore_benchmarks);
criterion_main!(benches);
