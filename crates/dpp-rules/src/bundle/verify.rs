//! Fail-closed bundle verification: signature authenticity + content integrity.
//!
//! Verification is two independent checks: **authenticity** (the manifest JWS
//! verifies under the pinned publisher key) and **integrity** (`content`
//! hashes to the value the signed manifest commits to). The signature check
//! itself is delegated to a caller-supplied [`JwsVerify`] rather than
//! implemented here, so this crate never depends on a JWS/crypto crate (which
//! would create a dependency cycle back through `dpp-domain`) and never grows
//! a second, drifting copy of signature-verification code. Content hashing is
//! delegated for the same anti-duplication reason, to [`crate::canonical`],
//! which is shared with every other integrity-hash consumer.
//!
//! The publisher key is pinned together with its algorithm, as a [`PinnedKey`],
//! and the algorithm is not fixed in [`JwsVerify`]. The update channel can move
//! to another signature algorithm, a post-quantum one included, by re-pinning
//! the key under that algorithm, without a change to the trait. The pin decides
//! the algorithm, never the bundle: [`verify_bundle`] requires the manifest
//! header's `alg` to equal the pin exactly. `EdDSA` and `Ed25519` are therefore
//! not interchangeable for a bundle. RFC 9864 section 4.1 registers the two
//! identifiers separately, so a pin names the identifier the publisher writes,
//! and a publisher that moves to the other one needs a re-pin.

use base64::Engine;

use super::types::{
    AcceptancePolicy, RulesetAcceptance, RulesetError, RulesetManifest, SignedBundle,
};

/// A publisher public key and the one algorithm it is pinned under.
///
/// `alg` is the JOSE `alg` identifier the publisher writes for that key, and the
/// manifest's protected header must name it exactly. `public_key_b64` is the key,
/// base64url-encoded. Both are borrowed from the caller's configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinnedKey<'a> {
    /// The JOSE `alg` identifier the key is pinned under, matched exactly against
    /// the manifest header.
    pub alg: &'a str,
    /// The base64url-encoded public key.
    pub public_key_b64: &'a str,
}

/// Verifies a compact JWS under a pinned publisher key. Implemented by the
/// caller (e.g. a thin adapter over `dpp-crypto`'s JWS verifier) and passed into
/// [`verify_bundle`].
///
/// [`verify_bundle`] refuses a manifest whose protected header `alg` is not
/// exactly `key.alg` before it calls this method, so an implementation only ever
/// sees the pinned identifier. An implementation must still verify under exactly
/// `key.alg`, using `key.public_key_b64`, and should refuse a JWS whose header
/// names another algorithm too: that is the only check when the method is called
/// directly, not through [`verify_bundle`].
///
/// Returns `Ok(true)` iff `jws` is a validly signed compact JWS under `key`.
/// `Ok(false)` for a well-formed JWS that does not verify, including one refused
/// because its header names another algorithm. `Err` for input the verifier
/// cannot parse, and for a pin it cannot verify under, such as an algorithm it
/// does not implement or a key that does not decode.
pub trait JwsVerify {
    /// Whether `jws` verifies under `key`, as described on the trait.
    fn verify(&self, jws: &str, key: &PinnedKey<'_>) -> Result<bool, RulesetError>;
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

/// Verify a bundle against the pinned publisher key and the caller's
/// [`AcceptancePolicy`].
///
/// Four checks, in this order, each fail-closed:
///
/// 1. **Authenticity** — the manifest's header names the pinned `alg`, and the
///    manifest JWS verifies under the pinned key.
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
    key: &PinnedKey<'_>,
    verifier: &dyn JwsVerify,
    policy: &AcceptancePolicy<'_>,
) -> Result<RulesetAcceptance, RulesetError> {
    // (1) Authenticity. The header must name the pinned identifier exactly. It is
    // read and compared, never trusted, and a mismatch is refused before the
    // verifier runs, so the verifier only ever sees the pinned identifier.
    if header_alg(&bundle.manifest_jws)? != key.alg {
        return Err(RulesetError::BadSignature);
    }
    // The manifest JWS verifies under the pinned key.
    if !verifier.verify(&bundle.manifest_jws, key)? {
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

/// The `alg` the protected header of a compact JWS names. Nothing is verified
/// here: the caller compares it with the pin.
///
/// # Errors
/// [`RulesetError::Malformed`] when the header is not base64url, is not a JSON
/// object, or has no string `alg`.
fn header_alg(jws: &str) -> Result<String, RulesetError> {
    let header_b64 = jws.split('.').next().unwrap_or_default();
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(header_b64)
        .map_err(|e| RulesetError::Malformed(format!("header base64: {e}")))?;
    let header: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&bytes)
        .map_err(|e| RulesetError::Malformed(format!("header is not a JSON object: {e}")))?;
    header
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| RulesetError::Malformed("JWS header has no string `alg`".into()))
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
