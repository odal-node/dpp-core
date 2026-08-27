//! Fail-closed bundle verification: signature authenticity + content integrity.
//!
//! Verification is two independent checks: **authenticity** (the manifest JWS
//! verifies under the pinned publisher key) and **integrity** (`content`
//! hashes to the value the signed manifest commits to). The EdDSA signature
//! check itself is delegated to a caller-supplied [`JwsVerify`] rather than
//! implemented here, so this crate never depends on a JWS/crypto crate (which
//! would create a dependency cycle back through `dpp-domain`) and never grows
//! a second, drifting copy of signature-verification code. Content hashing is
//! delegated for the same anti-duplication reason, to [`crate::canonical`],
//! which is shared with every other integrity-hash consumer.

use base64::Engine;

use super::types::{
    AcceptancePolicy, RulesetAcceptance, RulesetError, RulesetManifest, SignedBundle,
};

/// Verifies a compact EdDSA JWS against a base64url-encoded Ed25519 public
/// key. Implemented by the caller (e.g. a thin adapter over `dpp-crypto`'s
/// JWS verifier) and passed into [`verify_bundle`].
pub trait JwsVerify {
    /// Returns `Ok(true)` iff `jws` is a validly signed EdDSA compact JWS
    /// under `public_key_b64`. `Ok(false)` for a well-formed but invalid
    /// signature; `Err` only for malformed input the verifier cannot parse.
    fn verify_eddsa(&self, jws: &str, public_key_b64: &str) -> Result<bool, RulesetError>;
}

/// Canonical SHA-256 (hex) of a content value (RFC 8785 / JCS bytes), in this
/// module's error type.
///
/// Exposed so a signer builds the exact same `content_sha256` a verifier will
/// later check against. The hashing itself lives in [`crate::canonical`] and is
/// shared with every other integrity-hash consumer — a consumer outside the
/// ruleset channel should call that directly rather than adopt
/// [`RulesetError`], and must not re-implement it.
///
/// # Errors
/// [`RulesetError::Malformed`] if `content` cannot be JCS-canonicalised — RFC
/// 8785 rejects non-finite floats (`NaN`/`Infinity`), which can appear in a
/// bundle's unauthenticated `content` deserialized straight off the wire.
pub fn content_hash(content: &serde_json::Value) -> Result<String, RulesetError> {
    crate::canonical::content_hash(content)
        .map_err(|e| RulesetError::Malformed(format!("JCS canonicalisation failed: {e}")))
}

/// Verify a bundle against the pinned publisher public key (base64url) and the
/// caller's [`AcceptancePolicy`].
///
/// Four checks, in this order, each fail-closed:
///
/// 1. **Authenticity** — the manifest JWS verifies under the pinned key.
/// 2. **Integrity** — `content` hashes to what the signed manifest commits to.
/// 3. **Applicability** — the manifest's `effective_date` has arrived.
/// 4. **Currency** — the bundle is not older than the ruleset in force.
///
/// # Why 3 and 4 exist
///
/// They did not until 2026-08-27, and their absence was the gap a signature
/// cannot close. A signature is a statement about *origin*, permanent by
/// construction: it stays valid for as long as the key does. So an authentic
/// bundle whose rules start next year was adopted immediately, and — the one
/// with an adversary — an authentic bundle from two years ago replaced a newer
/// one, letting anyone able to serve bytes pin a node to superseded rules
/// without forging anything. This module's own premise is that a customer can
/// verify currency; nothing verified currency.
///
/// **Currency is measured on `effective_date`, not on `bundle_version`.**
/// Versions here are publisher-chosen strings like `"2026-Q3.1"`, and ordering
/// them lexicographically puts `2026-Q3.10` before `2026-Q3.2`. The effective
/// date is already in the signed manifest, already a point in time, and already
/// the thing the ordering is meant to express.
///
/// # Errors
/// [`RulesetError`] — fail-closed on a bad signature, hash mismatch, a bundle
/// that is not yet effective, one that has been superseded, or malformed input.
pub fn verify_bundle(
    bundle: &SignedBundle,
    publisher_pubkey_b64: &str,
    verifier: &dyn JwsVerify,
    policy: &AcceptancePolicy<'_>,
) -> Result<RulesetAcceptance, RulesetError> {
    // (1) Authenticity: the manifest JWS verifies under the pinned key.
    if !verifier.verify_eddsa(&bundle.manifest_jws, publisher_pubkey_b64)? {
        return Err(RulesetError::BadSignature);
    }
    // (2) The manifest is now trusted — extract it from the JWS payload.
    let manifest: RulesetManifest = decode_jws_payload(&bundle.manifest_jws)?;
    // (3) Integrity: content must hash to what the signed manifest commits to.
    if content_hash(&bundle.content)? != manifest.content_sha256 {
        return Err(RulesetError::ContentHashMismatch);
    }
    // (4) Applicability. Checked before currency so a future-dated bundle is
    // reported as future-dated rather than as superseded, which is the answer
    // that tells a caller to hold it and re-offer it later.
    if manifest.effective_date > policy.now {
        return Err(RulesetError::NotYetEffective {
            bundle_version: manifest.bundle_version,
            effective_date: manifest.effective_date,
            now: policy.now,
        });
    }
    // (5) Currency. Equal dates are accepted: a republish at the same effective
    // date is a correction, not a rollback, and refusing it would leave no way
    // to ship one.
    if let Some(in_force) = policy.in_force
        && manifest.effective_date < in_force.effective_date
    {
        return Err(RulesetError::Superseded {
            offered_version: manifest.bundle_version,
            offered: manifest.effective_date,
            in_force_version: in_force.bundle_version.clone(),
            in_force: in_force.effective_date,
        });
    }
    Ok(RulesetAcceptance::verified(
        manifest,
        bundle.content.clone(),
    ))
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
