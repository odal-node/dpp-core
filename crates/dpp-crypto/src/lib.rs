//! `dpp-crypto` — cryptographic primitives: Ed25519 signing, JWS compact
//! serialisation, and the encrypted keystore.
//!
//! All modules are pure (no I/O, no network). The crate compiles to `std` for
//! the node and to `wasm32` (where conditional) for plugin guests.
//!
//! Verifiable Credentials, `did:web` documents and status lists are
//! [`dpp-vc`]; the per-field disclosure contract is `dpp_domain::access`.
//! Signing bytes is a different job from deciding whose signature means what,
//! and a different job again from deciding which fields a role may see.
//!
//! With those gone this crate has **no workspace dependencies** — it is a leaf.
//!
//! [`dpp-vc`]: https://docs.rs/dpp-vc

pub mod jades;
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

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
