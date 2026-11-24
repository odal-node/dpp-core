//! AES-256-GCM encrypted Ed25519 key store with Argon2id key derivation.
//!
//! Keys are stored as JSON on disk, encrypted per-record with a unique nonce.
//! Rotation archives the current key and generates a fresh one; revocation
//! marks a key as compromised so it is excluded from the published DID document.
//!
//! ## Module layout
//!
//! - `backend`: [`KeyBackend`], where signing keys live, which [`KeyStore`] implements.
//! - `signer`: [`KeySigner`], the private half of a key that is not held in memory.
//! - `memory`: `MemoryKeyStore`, a second backend, behind the `test-utils` feature.
//! - `entry`: [`KeyEntry`], the key handed back to callers, which signs.
//! - `error`: [`PrivateJwkError`], why a JWK was refused as a private key, and
//!   [`KeyStoreLocked`], why a store was refused because it is open elsewhere.
//! - `lock`: the exclusive lock on `<store file>.lock` that gives a store one handle.
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
//!
//! ## One handle per file
//!
//! Either door takes an exclusive lock on `<store file>.lock` and holds it for the
//! life of the handle, so a second open of the same file, from this process or
//! another, is refused at once with [`KeyStoreLocked`]. A handle's in-memory records
//! are then the file's, and no write can discard another handle's key. See
//! [`KeyStore`] for why the lock is on a sibling file, and for what it does not stop.

mod backend;
#[cfg(test)]
mod backend_tests;
mod crypto;
mod entry;
mod error;
mod lock;
#[cfg(test)]
mod lock_tests;
#[cfg(any(test, feature = "test-utils"))]
mod memory;
#[cfg(test)]
mod memory_tests;
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
mod signer;
#[cfg(test)]
mod signer_tests;
mod store;
#[cfg(test)]
mod tests;

pub use backend::KeyBackend;
pub use entry::KeyEntry;
#[cfg(test)]
pub(crate) use entry::SigningMaterial;
pub use error::{KeyRevoked, KeyStoreLocked, PrivateJwkError, SignError};
#[cfg(any(test, feature = "test-utils"))]
pub use memory::MemoryKeyStore;
pub use signer::KeySigner;
pub use store::{KeyStore, PublicKeyInfo, UpgradeNeeded};
