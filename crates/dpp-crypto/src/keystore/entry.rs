//! [`KeyEntry`] — a decrypted, in-memory Ed25519 key pair handed back by the store.

use ed25519_dalek::{SigningKey, VerifyingKey};

pub struct KeyEntry {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub fingerprint: String,
    /// Whether this key has been revoked (see `KeyRecord::revoked`).
    pub revoked: bool,
}

impl Drop for KeyEntry {
    fn drop(&mut self) {
        // zeroize is called automatically by ed25519_dalek's Drop impl
    }
}
