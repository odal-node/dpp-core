//! Snapshot bound verification: what holds, what does not, and what a stripped
//! bound looks like.

use base64::Engine;
use chrono::{Duration, SecondsFormat, SubsecRound, Utc};
use dpp_crypto::keystore::KeyStore;
use serde_json::{Value, json};

use super::*;
use crate::test_support::temp_store;

const KEY: &str = "snapshot-signer";

fn public_key_b64(store: &KeyStore) -> String {
    let entry = store.load_key(KEY).expect("load key");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(entry.verifying_key.to_bytes())
}

/// The public view a snapshot wraps: passport fields plus the publish-time
/// proof, which is frozen and travels untouched.
fn public_view() -> Value {
    json!({
        "id": "0192f3c0-0000-7000-8000-000000000000",
        "productName": "Widget",
        "status": "active",
        "publicJwsSignature": "eyJhbGciOiJFZERTQSJ9.eyJmcm96ZW4iOnRydWV9.c2ln",
    })
}

/// Build a snapshot the way the publisher does: stamp the two timestamps, sign
/// the whole document, then attach the proof under the one key it does not
/// cover.
fn snapshot(store: &KeyStore, as_of: chrono::DateTime<Utc>, valid_for: Duration) -> Value {
    let rfc3339 = |t: chrono::DateTime<Utc>| t.to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut document = public_view();
    let object = document.as_object_mut().expect("object");
    object.insert("asOf".to_owned(), json!(rfc3339(as_of)));
    object.insert("validUntil".to_owned(), json!(rfc3339(as_of + valid_for)));

    let proof = dpp_crypto::jws::sign(store, KEY, &document).expect("sign snapshot");
    document
        .as_object_mut()
        .expect("object")
        .insert("snapshotJwsSignature".to_owned(), json!(proof));
    document
}

/// Rule 1 of the design: this is a pure addition. Every live read and every
/// passport signed before bounds existed has no outer proof, and must verify
/// exactly as it did before this module existed.
#[test]
fn a_document_with_no_outer_proof_is_unbound() {
    let store = temp_store("snapshot-unbound", KEY);
    let bound = verify_snapshot_bound(&public_view(), &public_key_b64(&store), Utc::now());

    assert_eq!(bound, SnapshotBound::Absent);
    assert!(!bound.is_expired());
    assert_eq!(bound.proven(), None);
}

#[test]
fn a_snapshot_inside_its_bound_is_current() {
    let store = temp_store("snapshot-current", KEY);
    let as_of = Utc::now();
    let document = snapshot(&store, as_of, Duration::days(7));

    let bound = verify_snapshot_bound(
        &document,
        &public_key_b64(&store),
        as_of + Duration::days(1),
    );

    assert!(matches!(bound, SnapshotBound::Current { .. }), "{bound:?}");
    assert!(!bound.is_expired());
    assert!(bound.proven().is_some());
}

/// The defect this whole module exists to remove: before it, a snapshot past
/// its own stated deadline verified as valid, because the deadline was stated
/// and signed and then read by nobody.
#[test]
fn a_snapshot_past_its_bound_is_expired() {
    let store = temp_store("snapshot-expired", KEY);
    let as_of = Utc::now() - Duration::days(30);
    let document = snapshot(&store, as_of, Duration::days(7));

    let bound = verify_snapshot_bound(&document, &public_key_b64(&store), Utc::now());

    let SnapshotBound::Expired {
        as_of: reported,
        valid_until,
    } = bound.clone()
    else {
        panic!("expected Expired, got {bound:?}");
    };
    assert!(bound.is_expired());
    // `asOf` is the thing a caller reports to whoever is looking at a stale
    // document, so it has to survive the verdict rather than be swallowed by
    // it. Whole seconds, because the publisher truncates at the source.
    assert_eq!(reported, as_of.trunc_subsecs(0));
    assert_eq!(valid_until, (as_of + Duration::days(7)).trunc_subsecs(0));
}

/// Expiry and tampering are different answers and must not arrive wearing the
/// same label: the signatures on an expired snapshot are fine, and sending a
/// reader to look for tampering that did not happen is its own harm.
#[test]
fn an_expired_snapshot_is_not_reported_as_tampering() {
    let store = temp_store("snapshot-expired-not-tampered", KEY);
    let as_of = Utc::now() - Duration::days(30);
    let document = snapshot(&store, as_of, Duration::days(7));

    let bound = verify_snapshot_bound(&document, &public_key_b64(&store), Utc::now());

    assert!(
        !matches!(bound, SnapshotBound::Unproven(_)),
        "an intact but stale snapshot must not be reported as unproven: {bound:?}"
    );
}

/// The attack the outer proof exists to stop: hold a copy, push its deadline
/// out, keep serving. The date is inside what the proof covers, so editing it
/// breaks the proof rather than extending the bound.
#[test]
fn a_forged_valid_until_does_not_extend_the_bound() {
    let store = temp_store("snapshot-forged-deadline", KEY);
    let as_of = Utc::now() - Duration::days(30);
    let mut document = snapshot(&store, as_of, Duration::days(7));

    document.as_object_mut().expect("object").insert(
        "validUntil".to_owned(),
        json!((Utc::now() + Duration::days(365)).to_rfc3339_opts(SecondsFormat::Secs, true)),
    );

    let bound = verify_snapshot_bound(&document, &public_key_b64(&store), Utc::now());

    assert!(
        matches!(bound, SnapshotBound::Unproven(_)),
        "a rewritten deadline must break the proof, not extend the bound: {bound:?}"
    );
    assert!(!bound.is_expired());
    assert_eq!(bound.proven(), None);
}

/// The bound travels in the outer proof precisely so the frozen publish-time
/// proof does not have to move. That only holds if the outer one covers it —
/// otherwise a copy could carry someone else's content proof.
#[test]
fn the_outer_proof_covers_the_publish_time_proof() {
    let store = temp_store("snapshot-covers-inner", KEY);
    let mut document = snapshot(&store, Utc::now(), Duration::days(7));

    document.as_object_mut().expect("object").insert(
        "publicJwsSignature".to_owned(),
        json!("eyJ4Ijoic3dhcHBlZCJ9"),
    );

    let bound = verify_snapshot_bound(&document, &public_key_b64(&store), Utc::now());

    assert!(matches!(bound, SnapshotBound::Unproven(_)), "{bound:?}");
}

#[test]
fn a_tampered_passport_field_leaves_the_bound_unproven() {
    let store = temp_store("snapshot-tampered-field", KEY);
    let mut document = snapshot(&store, Utc::now(), Duration::days(7));

    document
        .as_object_mut()
        .expect("object")
        .insert("productName".to_owned(), json!("Something Else"));

    let bound = verify_snapshot_bound(&document, &public_key_b64(&store), Utc::now());

    assert!(matches!(bound, SnapshotBound::Unproven(_)), "{bound:?}");
}

#[test]
fn a_proof_from_another_key_is_unproven() {
    let signer = temp_store("snapshot-wrong-key-signer", KEY);
    let other = temp_store("snapshot-wrong-key-other", KEY);
    let document = snapshot(&signer, Utc::now(), Duration::days(7));

    let bound = verify_snapshot_bound(&document, &public_key_b64(&other), Utc::now());

    assert!(matches!(bound, SnapshotBound::Unproven(_)), "{bound:?}");
}

/// Stripping the proof and keeping the dates gives `Absent`, not `Expired` —
/// the dates are text nobody vouched for, and `Absent` says exactly that.
///
/// This is the shape's sharpest edge, so it is pinned rather than left to be
/// discovered: a caller that reads `Absent` as "fine, no bound claimed" is
/// correct for a live read and wrong for a copy off the static tier, and only
/// the caller knows which it fetched.
#[test]
fn a_stripped_proof_leaves_the_dates_vouched_for_by_nobody() {
    let store = temp_store("snapshot-stripped", KEY);
    let as_of = Utc::now() - Duration::days(30);
    let mut document = snapshot(&store, as_of, Duration::days(7));

    document
        .as_object_mut()
        .expect("object")
        .remove("snapshotJwsSignature");

    let bound = verify_snapshot_bound(&document, &public_key_b64(&store), Utc::now());

    assert_eq!(bound, SnapshotBound::Absent);
    assert!(
        document.get("validUntil").is_some(),
        "the date is still there"
    );
    assert_eq!(
        bound.proven(),
        None,
        "an unvouched date must never be returned as a proven bound"
    );
}

/// The tolerance is a stated number, so it gets a test at its own edge rather
/// than a comment saying it is generous.
#[test]
fn the_expiry_edge_is_the_stated_skew_tolerance() {
    let store = temp_store("snapshot-skew", KEY);
    let as_of = Utc::now().trunc_subsecs(0);
    let document = snapshot(&store, as_of, Duration::days(7));
    let key = public_key_b64(&store);
    let deadline = as_of + Duration::days(7);

    let at_the_edge = verify_snapshot_bound(&document, &key, deadline + CLOCK_SKEW_TOLERANCE);
    assert!(
        matches!(at_the_edge, SnapshotBound::Current { .. }),
        "a clock exactly one tolerance fast must still see the snapshot as current: {at_the_edge:?}"
    );

    let past_it = verify_snapshot_bound(
        &document,
        &key,
        deadline + CLOCK_SKEW_TOLERANCE + Duration::seconds(1),
    );
    assert!(past_it.is_expired(), "{past_it:?}");
}

/// The contract is checked by **recomputing** canonical bytes from the parsed
/// document, so it rests on the document surviving a serialise/parse round trip
/// to the same JCS form. That is how a snapshot actually arrives — the
/// publisher signs a value, writes it as bytes, and a verifier parses those
/// bytes back — and the assumption is worth a test rather than an argument.
///
/// The awkward values are the point: a float, a large integer, non-ASCII text,
/// an escape, a nested object and an empty array are where a canonicaliser and
/// a parser disagree if they are going to.
#[test]
fn a_snapshot_survives_the_round_trip_through_bytes() {
    let store = temp_store("snapshot-round-trip", KEY);
    let as_of = Utc::now();
    let rfc3339 = as_of.to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut document = json!({
        "id": "0192f3c0-0000-7000-8000-000000000000",
        "productName": "Wäsche-Trockner \"Pro\"",
        "co2ePerUnitKg": 1.8,
        "nominalCapacityAh": 100.0,
        "serial": 9_007_199_254_740_991_i64,
        "materials": [],
        "manufacturer": { "name": "Ünïcode GmbH", "country": "DE" },
        "publicJwsSignature": "eyJhbGciOiJFZERTQSJ9.eyJmcm96ZW4iOnRydWV9.c2ln",
        "asOf": rfc3339,
        "validUntil": (as_of + Duration::days(7)).to_rfc3339_opts(SecondsFormat::Secs, true),
    });
    let proof = dpp_crypto::jws::sign(&store, KEY, &document).expect("sign snapshot");
    document
        .as_object_mut()
        .expect("object")
        .insert("snapshotJwsSignature".to_owned(), json!(proof));

    // Through bytes and back, the way it reaches a verifier.
    let bytes = serde_json::to_vec(&document).expect("serialise snapshot");
    let parsed: Value = serde_json::from_slice(&bytes).expect("parse snapshot");

    let bound = verify_snapshot_bound(&parsed, &public_key_b64(&store), as_of + Duration::days(1));

    assert!(
        matches!(bound, SnapshotBound::Current { .. }),
        "a snapshot must still verify after the round trip it actually makes: {bound:?}"
    );
}

#[test]
fn a_non_object_document_is_unproven_rather_than_unbound() {
    let store = temp_store("snapshot-not-an-object", KEY);
    let bound = verify_snapshot_bound(
        &json!(["not", "a", "snapshot"]),
        &public_key_b64(&store),
        Utc::now(),
    );

    assert!(matches!(bound, SnapshotBound::Unproven(_)), "{bound:?}");
}

#[test]
fn a_non_string_proof_is_unproven() {
    let store = temp_store("snapshot-proof-not-a-string", KEY);
    let mut document = snapshot(&store, Utc::now(), Duration::days(7));

    document
        .as_object_mut()
        .expect("object")
        .insert("snapshotJwsSignature".to_owned(), json!(42));

    let bound = verify_snapshot_bound(&document, &public_key_b64(&store), Utc::now());

    assert!(matches!(bound, SnapshotBound::Unproven(_)), "{bound:?}");
}
