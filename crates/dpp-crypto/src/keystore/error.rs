//! Errors of the keystore module.
//!
// LAYOUT-DEVIATION: rule 1 counts three public types as a file that is no longer about
// one thing, but it is a rule about types that have gravity. None of these four has an
// `impl` block or a serde derive, and each runs 15 to 30 lines with its docs. Each is one
// way the key store refuses, and rule 14 asks for a module's failure modes to be read in
// one file without reading the module.

/// Why a JWK was refused as a private key.
///
/// One variant per rule, so a caller and each test can tell the refusals apart. No
/// variant carries a value from the JWK, so an error never prints key material.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PrivateJwkError {
    /// No `d` member: the JWK describes a public key.
    #[error("not a private key: the JWK has no `d` member")]
    NotPrivate,
    /// `kty` and `crv` name no supported key: `OKP` with `Ed25519`, or `EC` with
    /// `P-256`, `P-384` or `P-521`.
    #[error("unsupported key: `kty` and `crv` name no supported key type and curve")]
    UnsupportedKey,
    /// `alg` is present and is not a string that names the key's algorithm.
    #[error("the `alg` member does not name the key's algorithm")]
    AlgorithmMismatch,
    /// The public members `x` (and `y`) are missing, malformed, of the wrong length,
    /// or not a point on the curve.
    #[error("the public members are not a valid public key")]
    PublicKeyInvalid,
    /// `d` is not base64url, is not the curve's field size, or is not a valid private
    /// key. For an EC curve that means zero, or at or above the group order. An Ed25519
    /// `d` is a 32-octet seed (RFC 8037 §2), and any 32 octets are one, so only the
    /// length applies to it.
    #[error("the private member `d` is not a valid private key")]
    PrivateKeyInvalid,
    /// The public key that `d` derives is not the public key in `x` and `y`.
    #[error("the private member `d` does not belong to the JWK's public key")]
    KeyPairMismatch,
}

/// A key store file is already open, in this process or another.
///
/// [`KeyStore::open`](super::KeyStore::open) and
/// [`KeyStore::open_and_migrate`](super::KeyStore::open_and_migrate) hold an exclusive
/// lock on a file beside the store for as long as the handle lives, so a store has one
/// writer. They are returned inside an `anyhow::Error`: recover this with
/// `error.downcast_ref::<KeyStoreLocked>()`.
///
/// The open is refused at once and is not retried. A host that restarts while the old
/// process is still shutting down opens the store again itself, once the old handle is
/// gone.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "key store is open elsewhere: {} is locked by another handle, in this process or \
     another, and a store has one writer at a time",
    .lock_path.display()
)]
#[non_exhaustive]
pub struct KeyStoreLocked {
    /// The lock file, `<store file>.lock`, beside the store file once any symlink to it
    /// is resolved.
    pub lock_path: std::path::PathBuf,
}

/// A revoked key was asked to sign.
///
/// A key is revoked when it is compromised, and a revoked key signs nothing. Every
/// signature this crate makes passes through [`KeyEntry::sign`](super::KeyEntry::sign),
/// which refuses one. It reads the
/// `revoked` flag the entry carries, which [`KeyStore::load_key`](super::KeyStore::load_key)
/// copies when it loads: an entry loaded before the revocation keeps its copy and
/// still signs, so load the key again after revoking it. The remedy is a new current key:
/// [`KeyStore::revoke_and_rotate`](super::KeyStore::revoke_and_rotate) issues one and
/// keeps the revoked key archived as revoked, so it stays out of every published key set.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("key {fingerprint} is revoked; give it a successor with `KeyStore::revoke_and_rotate`")]
#[non_exhaustive]
pub struct KeyRevoked {
    /// The revoked key's fingerprint, the hex SHA-256 of its public key.
    pub fingerprint: String,
}

/// Why a key did not sign.
///
/// [`KeyEntry::sign`](super::KeyEntry::sign) returns it, and so does a
/// [`KeySigner`](super::KeySigner), whose key may sit behind a device or a service
/// that fails for reasons a key held in memory does not.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SignError {
    /// The key is revoked, and a revoked key signs nothing.
    #[error(transparent)]
    Revoked(#[from] KeyRevoked),
    /// The backend that holds the key could not sign: the device or service was not
    /// reachable, or refused. The text is the backend's own and carries no key material.
    #[error("the key could not sign: {0}")]
    Backend(String),
}
