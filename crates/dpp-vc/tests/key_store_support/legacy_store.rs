//! A key store file whose current key is revoked, in the format key stores had
//! before they carried an integrity tag.
//!
//! A store's own calls never leave its current key revoked, and a store written
//! today seals the flag under its integrity tag. A store written before the tag
//! existed carries the flag in plaintext, and `KeyStore::open_and_migrate` opens it,
//! re-encrypts it and seals it as it finds it. That is how a revoked current key
//! reaches the document builders, so it is how the tests here make one.
//!
//! The file is the oldest format `open_and_migrate` reads: a JSON map from key name
//! to record, each private key encrypted with AES-256-GCM under the SHA-256 of the
//! passphrase, with no associated data. The Ed25519 keys are generated when the store
//! is written and are not kept, so no key material is committed.

use std::path::PathBuf;

use aes_gcm::aead::{Aead, KeyInit, consts::U12};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use ed25519_dalek::SigningKey;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

/// The passphrase every store here is written under.
pub const PASSPHRASE: &str = "test-pass";

/// An Ed25519 key pair made for one store and thrown away with it.
struct TestKey {
    secret: [u8; 32],
    public: [u8; 32],
}

impl TestKey {
    fn generate() -> Self {
        let key = SigningKey::generate(&mut rand::rand_core::UnwrapErr(rand::rngs::SysRng));
        Self {
            secret: key.to_bytes(),
            public: key.verifying_key().to_bytes(),
        }
    }
}

/// One record: `key`'s private half encrypted under [`PASSPHRASE`], its public half
/// and fingerprint in plaintext beside it, and `revoked` as given.
fn record(key: &TestKey, revoked: bool, nonce: [u8; 12]) -> Value {
    let digest = Sha256::digest(PASSPHRASE.as_bytes());
    let cipher_key = Key::<Aes256Gcm>::try_from(digest.as_slice()).expect("a 32-byte key");
    let cipher = Aes256Gcm::new(&cipher_key);
    let encrypted = cipher
        .encrypt(<&Nonce<U12>>::from(&nonce), key.secret.as_slice())
        .expect("encrypt the private key");

    json!({
        "encrypted_signing_key": encrypted,
        "nonce": nonce.to_vec(),
        "fingerprint": hex::encode(Sha256::digest(key.public)),
        "verifying_key_hex": hex::encode(key.public),
        "revoked": revoked,
        "algorithm": "EdDSA",
    })
}

/// Write a store at a fresh temporary path whose current key under `key_id` is revoked,
/// beside an archived key that is not when `with_archived` is set, and return the
/// path. Both keys are generated here, one pair per call. Open it with
/// `KeyStore::open_and_migrate` and [`PASSPHRASE`].
pub fn with_revoked_current(label: &str, key_id: &str, with_archived: bool) -> PathBuf {
    let mut records = Map::new();
    records.insert(
        key_id.to_owned(),
        record(&TestKey::generate(), true, [1; 12]),
    );
    if with_archived {
        records.insert(
            format!("{key_id}#archived-0"),
            record(&TestKey::generate(), false, [2; 12]),
        );
    }

    let path = std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()));
    std::fs::write(&path, Value::Object(records).to_string()).expect("write the key store");
    path
}
