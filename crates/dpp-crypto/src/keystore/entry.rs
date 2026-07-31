//! [`KeyEntry`] — a decrypted, in-memory key pair handed back by the store.

use ed25519_dalek::{SigningKey, VerifyingKey};

use crate::jws::algorithm::KeyAlgorithm;

/// `algorithm` is carried alongside the key material so signing and
/// verification read it from the key rather than assuming it. The material
/// itself is Ed25519-typed because Ed25519 is the only algorithm this store
/// issues; the struct is `#[non_exhaustive]` so that can change without
/// breaking downstream construction.
#[non_exhaustive]
pub struct KeyEntry {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub fingerprint: String,
    /// Whether this key has been revoked (see `KeyRecord::revoked`).
    pub revoked: bool,
    pub algorithm: KeyAlgorithm,
}

impl Drop for KeyEntry {
    fn drop(&mut self) {
        // zeroize is called automatically by ed25519_dalek's Drop impl
    }
}
