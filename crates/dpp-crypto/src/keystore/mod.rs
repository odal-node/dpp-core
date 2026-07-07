//! AES-256-GCM encrypted Ed25519 key store with Argon2id key derivation.
//!
//! Keys are stored as JSON on disk, encrypted per-record with a unique nonce.
//! Rotation archives the current key and generates a fresh one; revocation
//! marks a key as compromised so it is excluded from the published DID document.
//!
//! ## Module layout
//!
//! - `entry` — [`KeyEntry`], the decrypted in-memory key handed back to callers.
//! - `store` — [`KeyStore`] itself: the encrypted record map and its
//!   open/generate/load/persist paths.
//! - `crypto`, `rotation`, `migration` — key derivation, rotation, and
//!   legacy-KDF migration, each already a focused file.

mod crypto;
mod entry;
mod migration;
mod rotation;
mod store;
#[cfg(test)]
mod tests;

pub use entry::KeyEntry;
pub use store::KeyStore;
