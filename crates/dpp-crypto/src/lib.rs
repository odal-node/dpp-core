//! `dpp-crypto` — cryptographic primitives: Ed25519 signing, JWS compact
//! serialisation, and the encrypted keystore.
//!
//! All modules are pure (no I/O, no network). The crate compiles to `std` for
//! the node and to `wasm32` (where conditional) for plugin guests.
//!
//! Verifiable Credentials, `did:web` documents and status lists moved to
//! [`dpp-vc`]: signing bytes is a different job from deciding whose signature
//! means what. `access` remains here pending the `dpp-domain` analysis — see
//! that module's own note.
//!
//! [`dpp-vc`]: https://docs.rs/dpp-vc

pub mod access;
pub mod jws;
pub mod keystore;

#[cfg(test)]
mod test_support;

/// Infallible OS randomness source, used for Ed25519 key generation and
/// AES-GCM nonce/salt generation. `SysRng` is fallible (`TryCryptoRng`);
/// `UnwrapErr` panics on the (practically unreachable) OS-entropy-source
/// failure instead of threading a `Result` through every call site.
pub(crate) fn os_rng() -> rand::rand_core::UnwrapErr<rand::rngs::SysRng> {
    rand::rand_core::UnwrapErr(rand::rngs::SysRng)
}

// ── Flat re-exports — maintain stable paths for external callers ─────────────

pub use access::{PolicyDecision, SectorAccessPolicy, filter_by_audience};
