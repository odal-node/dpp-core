//! Example: open a keystore, generate a signing key, and sign a payload as a
//! compact JWS.
//!
//! Run with: `cargo run -p dpp-crypto --example sign_and_verify`

use dpp_crypto::jws::{signer, verifier};
use dpp_crypto::keystore::KeyStore;
use serde_json::json;

fn main() {
    // `KeyStore::open` creates the file if it is absent, so the path is a real
    // decision: a deployment passes a configured location outside the source
    // tree. This example uses a temp path and removes it on the way out.
    let path = std::env::temp_dir().join(format!("odal-example-{}.json", std::process::id()));

    let store = KeyStore::open(&path, "correct-horse-battery-staple").expect("open keystore");
    store.generate_key("issuer").expect("generate key");

    println!("=== Keystore ===\n");
    println!("  path:     {}", path.display());
    println!("  kdf:      Argon2id, HMAC-SHA256 over the stored records");

    let public = store
        .public_key("issuer")
        .expect("the key was just generated");
    println!("  key:      issuer");
    println!("  public:   {}", public.verifying_key_hex);
    println!("  revoked:  {}", public.revoked);

    println!("\n=== Signing ===\n");

    let payload = json!({ "product": "battery", "status": "published" });
    let jws = signer::sign(&store, "issuer", &payload).expect("sign");

    // The kid travels in the JWS header so a verifier can pick the right key
    // out of a DID document that may list several — including rotated ones.
    let kid = verifier::extract_kid_from_jws(&jws).expect("kid present");
    println!("  payload:  {payload}");
    println!("  kid:      {kid}");
    println!("  jws:      {}…", &jws[..jws.len().min(48)]);

    let segments = jws.split('.').count();
    println!("  segments: {segments} (header.payload.signature)");

    std::fs::remove_file(&path).ok();
}
