//! AES-256-GCM encrypted Ed25519 key store with Argon2id key derivation.
//!
//! Keys are stored as JSON on disk, encrypted per-record with a unique nonce.
//! Rotation archives the current key and generates a fresh one; revocation
//! marks a key as compromised so it is excluded from the published DID document.
//!
//! ## Module layout
//!
//! - `entry` — [`KeyEntry`], the decrypted in-memory key handed back to callers.
//! - `error` — [`PrivateJwkError`], why a JWK was refused as a private key.
//! - `private_jwk` — [`KeyEntry::from_private_jwk`], which builds a key pair from the
//!   private JWK of a key someone else generated, in memory only.
//! - `store` — [`KeyStore`] itself: the encrypted record map and its
//!   open/generate/load/persist paths.
//! - `crypto`, `rotation`, `migration` — key derivation, rotation, and
//!   legacy-format migration, each already a focused file.
//!
//! ## Opening a store
//!
//! [`KeyStore::open`] refuses a store that predates any current security
//! property and names which one — see [`UpgradeNeeded`].
//! [`KeyStore::open_and_migrate`] opens it anyway, upgrades it in place, and
//! then re-opens strictly. Production callers want the second; it is a no-op for
//! a store already in the current format.

mod crypto;
mod entry;
mod error;
mod migration;
#[cfg(test)]
mod persist_tests;
mod private_jwk;
#[cfg(test)]
mod private_jwk_refusal_tests;
#[cfg(test)]
mod private_jwk_tests;
#[cfg(test)]
mod rfc9106_vector_tests;
mod rotation;
mod store;
#[cfg(test)]
mod tests;

pub use entry::KeyEntry;
#[cfg(test)]
pub(crate) use entry::SigningMaterial;
pub use error::PrivateJwkError;
pub use store::{KeyStore, PublicKeyInfo, UpgradeNeeded};
