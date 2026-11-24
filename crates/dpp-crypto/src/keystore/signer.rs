//! [`KeySigner`], the private half of one key pair, wherever it is kept.

use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::public_key::PublicKey;

use super::error::SignError;

/// The private half of one key pair, wherever it is kept.
///
/// A [`KeyEntry`](super::KeyEntry) signs through one, so a key held in memory and a key
/// that never leaves a device or a service are signed with by the same code. A
/// [`KeyBackend`](super::KeyBackend) whose keys are not in memory hands out a
/// [`KeyEntry`](super::KeyEntry) built with
/// [`KeyEntry::from_signer`](super::KeyEntry::from_signer).
///
/// **A signer is one key for its whole life.** [`algorithm`](Self::algorithm),
/// [`public_key`](Self::public_key) and [`sign`](Self::sign) all answer for the same
/// key, every time. That is what lets a caller take a token's `alg` and `kid` from the
/// signer and then sign with it, and know the signature is by the key the header names.
/// A signer that followed a rotation, so that a later call used a newer key, would
/// break that, and a backend that rotates hands out a new signer instead.
///
/// No key or signature size is assumed: [`algorithm`](Self::algorithm) says what the key
/// is, and the length of what [`sign`](Self::sign) returns follows from it.
pub trait KeySigner: Send + Sync {
    /// The algorithm the key signs under. It is the algorithm of
    /// [`public_key`](Self::public_key).
    fn algorithm(&self) -> KeyAlgorithm;

    /// The public half, with its algorithm.
    fn public_key(&self) -> PublicKey;

    /// Sign `message` and return the raw signature: for ECDSA the fixed-length `R ‖ S`
    /// with `S` in the lower half of its range, the form
    /// [`PublicKey::verify`](crate::jws::PublicKey::verify) reads.
    ///
    /// Revocation is not this method's to check: [`KeyEntry::sign`](super::KeyEntry::sign)
    /// refuses a revoked key before it asks the signer.
    ///
    /// # Errors
    ///
    /// [`SignError::Backend`] when the device or service that holds the key could not
    /// sign.
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignError>;
}
