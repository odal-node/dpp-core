use base64::Engine;
use criterion::{Criterion, criterion_group, criterion_main};
use dpp_crypto::{jws::signer, jws::verifier as jws_verifier, keystore::KeyStore};
use serde_json::json;

fn setup() -> (KeyStore, String, serde_json::Value, String, String) {
    let path = std::env::temp_dir().join(format!("bench-crypto-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "bench-pass").expect("open store");
    store.generate_key("bench").expect("generate key");

    let payload = json!({"passport_id": "bench-001", "status": "draft"});
    let jws = signer::sign(&store, "bench", &payload).expect("sign");

    let entry = store.load_key("bench").expect("load key");
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let pub_key_b64 = b64.encode(entry.verifying_key.as_bytes());

    (store, "bench".to_owned(), payload, jws, pub_key_b64)
}

fn crypto_benchmarks(c: &mut Criterion) {
    let (store, key_id, payload, jws, pub_key_b64) = setup();

    c.bench_function("jws_sign", |b| {
        b.iter(|| signer::sign(&store, &key_id, &payload).unwrap());
    });

    c.bench_function("jws_verify_via_store", |b| {
        b.iter(|| signer::verify(&store, &key_id, &jws).unwrap());
    });

    c.bench_function("jws_verify_standalone", |b| {
        b.iter(|| jws_verifier::verify_jws(&jws, &pub_key_b64).unwrap());
    });
}

criterion_group!(benches, crypto_benchmarks);
criterion_main!(benches);
