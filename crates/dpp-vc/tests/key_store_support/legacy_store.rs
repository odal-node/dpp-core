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
//! passphrase, with no associated data. The keys are the Ed25519 test vectors of
//! RFC 8032 §7.1, so nothing here generates key material.

use std::path::PathBuf;

use aes_gcm::aead::{Aead, KeyInit, consts::U12};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

/// The passphrase every store here is written under.
pub const PASSPHRASE: &str = "test-pass";

/// An Ed25519 key pair, given as hex.
pub struct TestKey {
    secret_hex: &'static str,
    public_hex: &'static str,
}

/// RFC 8032 §7.1, TEST 1.
pub const KEY_ONE: TestKey = TestKey {
    secret_hex: "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
    public_hex: "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
};

/// RFC 8032 §7.1, TEST 2.
pub const KEY_TWO: TestKey = TestKey {
    secret_hex: "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
    public_hex: "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
};

/// One record: `key`'s private half encrypted under [`PASSPHRASE`], its public half
/// and fingerprint in plaintext beside it, and `revoked` as given.
fn record(key: &TestKey, revoked: bool, nonce: [u8; 12]) -> Value {
    let digest = Sha256::digest(PASSPHRASE.as_bytes());
    let cipher_key = Key::<Aes256Gcm>::try_from(digest.as_slice()).expect("a 32-byte key");
    let cipher = Aes256Gcm::new(&cipher_key);
    let secret = hex::decode(key.secret_hex).expect("the secret is hex");
    let encrypted = cipher
        .encrypt(<&Nonce<U12>>::from(&nonce), secret.as_slice())
        .expect("encrypt the private key");
    let public = hex::decode(key.public_hex).expect("the public key is hex");

    json!({
        "encrypted_signing_key": encrypted,
        "nonce": nonce.to_vec(),
        "fingerprint": hex::encode(Sha256::digest(&public)),
        "verifying_key_hex": key.public_hex,
        "revoked": revoked,
        "algorithm": "EdDSA",
    })
}

/// Write a store at a fresh temporary path whose current key under `key_id` is
/// `current`, revoked, beside `archived` as an archived key that is not, and return
/// the path. Open it with `KeyStore::open_and_migrate` and [`PASSPHRASE`].
pub fn with_revoked_current(
    label: &str,
    key_id: &str,
    current: &TestKey,
    archived: Option<&TestKey>,
) -> PathBuf {
    let mut records = Map::new();
    records.insert(key_id.to_owned(), record(current, true, [1; 12]));
    if let Some(archived) = archived {
        records.insert(
            format!("{key_id}#archived-0"),
            record(archived, false, [2; 12]),
        );
    }

    let path = std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()));
    std::fs::write(&path, Value::Object(records).to_string()).expect("write the key store");
    path
}
