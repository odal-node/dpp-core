//! Fail-closed bundle verification: signature authenticity + content integrity.
//!
//! Verification is two independent checks: **authenticity** (the manifest JWS
//! verifies under the pinned publisher key) and **integrity** (`content`
//! hashes to the value the signed manifest commits to). The EdDSA signature
//! check itself is delegated to a caller-supplied [`JwsVerify`] rather than
//! implemented here, so this crate never depends on a JWS/crypto crate (which
//! would create a dependency cycle back through `dpp-domain`) and never grows
//! a second, drifting copy of signature-verification code. Content hashing
//! (`sha2`) stays inline — it is deterministic and carries none of a
//! signature check's duplication risk.

use base64::Engine;
use sha2::{Digest, Sha256};

use super::types::{RulesetError, RulesetManifest, SignedBundle, VerifiedRuleset};

/// Verifies a compact EdDSA JWS against a base64url-encoded Ed25519 public
/// key. Implemented by the caller (e.g. a thin adapter over `dpp-crypto`'s
/// JWS verifier) and passed into [`verify_bundle`].
pub trait JwsVerify {
    /// Returns `Ok(true)` iff `jws` is a validly signed EdDSA compact JWS
    /// under `public_key_b64`. `Ok(false)` for a well-formed but invalid
    /// signature; `Err` only for malformed input the verifier cannot parse.
    fn verify_eddsa(&self, jws: &str, public_key_b64: &str) -> Result<bool, RulesetError>;
}

/// Canonical SHA-256 (hex) of a content value (RFC 8785 / JCS bytes).
///
/// Exposed so a signer builds the exact same `content_sha256` a verifier
/// will later check against — one hash function, two call sites.
#[must_use]
pub fn content_hash(content: &serde_json::Value) -> String {
    let bytes = serde_jcs::to_vec(content).expect("JCS canonicalisation is infallible");
    hex::encode(Sha256::digest(&bytes))
}

/// Verify a bundle against the pinned publisher public key (base64url). Both
/// the signature (authenticity) and the content hash (integrity) must pass.
///
/// # Errors
/// [`RulesetError`] — fail-closed on a bad signature, hash mismatch, or
/// malformed input.
pub fn verify_bundle(
    bundle: &SignedBundle,
    publisher_pubkey_b64: &str,
    verifier: &dyn JwsVerify,
) -> Result<VerifiedRuleset, RulesetError> {
    // (1) Authenticity: the manifest JWS verifies under the pinned key.
    if !verifier.verify_eddsa(&bundle.manifest_jws, publisher_pubkey_b64)? {
        return Err(RulesetError::BadSignature);
    }
    // (2) The manifest is now trusted — extract it from the JWS payload.
    let manifest: RulesetManifest = decode_jws_payload(&bundle.manifest_jws)?;
    // (3) Integrity: content must hash to what the signed manifest commits to.
    if content_hash(&bundle.content) != manifest.content_sha256 {
        return Err(RulesetError::ContentHashMismatch);
    }
    Ok(VerifiedRuleset {
        manifest,
        content: bundle.content.clone(),
    })
}

/// Decode the payload segment of a compact JWS into `T` (used only after the
/// signature verified, so the bytes are trusted).
fn decode_jws_payload<T: for<'de> serde::Deserialize<'de>>(jws: &str) -> Result<T, RulesetError> {
    let payload_b64 = jws
        .split('.')
        .nth(1)
        .ok_or_else(|| RulesetError::Malformed("JWS has no payload segment".into()))?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| RulesetError::Malformed(format!("payload base64: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| RulesetError::Malformed(format!("payload json: {e}")))
}
