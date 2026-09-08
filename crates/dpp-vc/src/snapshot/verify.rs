//! Verifying a continuity snapshot's time bound.

use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use dpp_crypto::jws::{canonical::canonicalize, verify_jws};
use serde_json::Value;

use super::bound::SnapshotBound;

/// The document key holding the outer proof.
const PROOF_KEY: &str = "snapshotJwsSignature";
/// The document key holding the instant this copy was taken.
const AS_OF_KEY: &str = "asOf";
/// The document key holding the instant this copy stops being good.
const VALID_UNTIL_KEY: &str = "validUntil";

/// How far a verifier's clock may run ahead before it starts rejecting
/// snapshots the node considers current.
///
/// The publisher writes `validUntil = asOf + 7 days` and re-signs a published
/// snapshot every 24 hours, so a copy is normally six days from its own
/// deadline and this tolerance costs nothing against that margin. It exists
/// because zero is the wrong number: with no tolerance, a verifier whose clock
/// is a few seconds fast rejects a snapshot that was renewed moments ago, and
/// the failure looks like expiry rather than like clock skew.
///
/// Stated as a constant rather than left implicit so that it is a number
/// someone can disagree with.
pub const CLOCK_SKEW_TOLERANCE: Duration = Duration::minutes(5);

/// Check the time bound on a continuity snapshot.
///
/// # The two proofs nest, they do not compete
///
/// A published passport's `publicJwsSignature` is frozen at publish and never
/// re-signed — it is pinned by hash elsewhere, so re-signing it on a refresh
/// cadence would fork the passport's public proof into two valid signed public
/// views with nothing to say which is the passport's. The bound therefore
/// travels in a *second*, outer proof over the whole document except itself,
/// `publicJwsSignature` included.
///
/// So the inner proof attests the passport's content and never expires; the
/// outer attests *this copy, taken then, good until then*. This function checks
/// only the outer one. Verifying the inner one is a separate question with a
/// separate answer, and a caller serving a passport needs both.
///
/// # What it does, in order
///
/// 1. **No outer proof ⇒ [`SnapshotBound::Absent`].** A pure addition: a
///    document without one is treated exactly as it was before bounds existed.
/// 2. **Present ⇒ verified content-bound**, against the document minus that one
///    field. A proof that does not verify, or that covers different bytes, is
///    [`SnapshotBound::Unproven`] — that is tampering, not expiry, and the two
///    must not arrive at the caller wearing the same label.
/// 3. **Then, and only then, the dates are read.** They are inside what the
///    outer proof covers, so before step 2 they are attacker-editable text on a
///    copy anyone can hold. An unproven `validUntil` is not a bound and this
///    never treats it as one.
///
/// `now` is a parameter rather than a call to the clock so the expiry boundary
/// is testable, matching `verify_credential_claims`.
#[must_use]
pub fn verify_snapshot_bound(
    document: &Value,
    public_key_b64: &str,
    now: DateTime<Utc>,
) -> SnapshotBound {
    let Some(object) = document.as_object() else {
        return SnapshotBound::Unproven("snapshot document is not a JSON object".to_owned());
    };

    let proof = match object.get(PROOF_KEY) {
        // Deliberately before any look at `asOf`/`validUntil`: whatever dates
        // this document carries, nothing has vouched for them, so the pair is
        // absent rather than merely unverified.
        None => return SnapshotBound::Absent,
        Some(Value::String(jws)) => jws,
        Some(_) => {
            return SnapshotBound::Unproven(format!("`{PROOF_KEY}` is present but not a string"));
        }
    };

    // The signature first, then what it covers. Either alone is worthless: a
    // valid signature over other bytes says nothing about this document, and
    // matching bytes with no valid signature say nothing about who produced
    // them.
    match verify_jws(proof, public_key_b64) {
        Ok(true) => {}
        Ok(false) => {
            return SnapshotBound::Unproven("snapshot proof does not verify".to_owned());
        }
        Err(e) => {
            return SnapshotBound::Unproven(format!("snapshot proof is malformed: {e}"));
        }
    }

    let mut covered = object.clone();
    covered.remove(PROOF_KEY);
    let Ok(expected) = canonicalize(&Value::Object(covered)) else {
        return SnapshotBound::Unproven("snapshot document cannot be canonicalized".to_owned());
    };
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let signed = proof.split('.').nth(1).and_then(|p| b64.decode(p).ok());
    if signed.as_deref() != Some(expected.as_slice()) {
        return SnapshotBound::Unproven(
            "snapshot proof covers different bytes than this document".to_owned(),
        );
    }

    let (Some(as_of), Some(valid_until)) = (
        rfc3339(object.get(AS_OF_KEY)),
        rfc3339(object.get(VALID_UNTIL_KEY)),
    ) else {
        // Reachable only from a publisher that signed a bound it did not state,
        // since these fields are inside what the proof just verified.
        return SnapshotBound::Unproven(format!(
            "snapshot proof verifies but `{AS_OF_KEY}`/`{VALID_UNTIL_KEY}` are missing or unreadable"
        ));
    };

    if now > valid_until + CLOCK_SKEW_TOLERANCE {
        SnapshotBound::Expired { as_of, valid_until }
    } else {
        SnapshotBound::Current { as_of, valid_until }
    }
}

/// Read an RFC 3339 instant from a JSON string field.
fn rfc3339(value: Option<&Value>) -> Option<DateTime<Utc>> {
    let text = value?.as_str()?;
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}
