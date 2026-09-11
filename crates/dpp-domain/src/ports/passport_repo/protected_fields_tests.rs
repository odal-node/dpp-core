//! Which passport fields a patch may never reach.

use super::protected_fields::PROTECTED_PATCH_FIELDS;
use crate::passport::PASSPORT_WIRE_KEYS;
use crate::test_support::fully_populated_passport;

/// Every protected key must be a key `Passport` actually serialises to.
///
/// The list is strings, and nothing else ties them to the struct. A renamed
/// field — or a typo — leaves an entry protecting a key that no longer
/// exists, and the field it was meant to protect silently becomes patchable.
/// That failure is invisible: the guard still runs, still finds nothing, and
/// still reports success.
///
/// [`fully_populated_passport`] is used rather than a locally built instance
/// because a field that is `None` does not appear in the JSON at all and would
/// look exactly like a stale entry. That helper exists to set every
/// `skip_serializing_if` field; a hand-built copy here was a second thing to
/// keep in step with the struct, and it fell behind the first time this list
/// grew.
#[test]
fn every_protected_key_is_a_real_passport_field() {
    let passport = fully_populated_passport();
    let json = serde_json::to_value(&passport).expect("passport serialises");
    let obj = json.as_object().expect("passport is a JSON object");

    let stale: Vec<&str> = PROTECTED_PATCH_FIELDS
        .iter()
        .copied()
        .filter(|k| !obj.contains_key(*k))
        .collect();

    assert!(
        stale.is_empty(),
        "PROTECTED_PATCH_FIELDS names keys `Passport` does not serialise to: {stale:?}\n\
         Either the field was renamed and this list was not, or \
         `fully_populated_passport` does not populate it. Both mean the field is \
         unprotected."
    );
}

/// No duplicates — a repeated entry is a sign the list was edited by hand in
/// two places, which is the failure mode this list exists to end.
#[test]
fn protected_keys_are_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for key in PROTECTED_PATCH_FIELDS {
        assert!(seen.insert(*key), "duplicate protected key: {key}");
    }
}

/// The envelope keys a free-form field patch may deliberately change.
///
/// Everything `Passport` serialises is either here or in
/// [`PROTECTED_PATCH_FIELDS`]. Nothing may be in both, and nothing may be in
/// neither — which is what
/// [`every_wire_key_is_classified`] enforces.
///
/// Six entries, and each earns its place:
///
/// - `productName`, `co2ePerUnit`, `repairabilityScore` and `productGroupData`
///   are content an operator supplies and may legitimately correct.
/// - `complianceResult` and `lintResult` are *recomputed* and written back by
///   the same path, not supplied by a caller. They are patchable because that
///   is the mechanism, not because a user edits them.
const DELIBERATELY_PATCHABLE: &[&str] = &[
    "co2ePerUnit",
    "complianceResult",
    "lintResult",
    "productGroupData",
    "productName",
    "repairabilityScore",
];

/// 🚨 Every serialised envelope key is classified, one way or the other.
///
/// `PROTECTED_PATCH_FIELDS` is a **deny-list**, so before this test a modelled
/// field was patchable unless someone remembered to add it. That default is why
/// ten envelope fields — `granularity`, `productGroup`, `productId`,
/// `commodityCode`, `placedOnMarketDate`, `responsibleOperator`, `batchId`,
/// `manufacturer`, `qrCodeUrl` and `updatedAt` — were writable by contract for
/// as long as they had existed. None was ever reachable through the consumer
/// that ships, because it builds its delta from an allow-list of its own; but
/// that guard lives in a consumer, and this list is what every other
/// implementor inherits.
///
/// Adding a field to `Passport` now fails here until it is deliberately placed
/// in one list or the other. **That is the point**: the previous default
/// answered the question by saying nothing, and a permission granted by silence
/// is the one nobody reviews.
#[test]
fn every_wire_key_is_classified() {
    let unclassified: Vec<&str> = PASSPORT_WIRE_KEYS
        .iter()
        .copied()
        .filter(|k| !PROTECTED_PATCH_FIELDS.contains(k) && !DELIBERATELY_PATCHABLE.contains(k))
        .collect();

    assert!(
        unclassified.is_empty(),
        "envelope keys are neither protected nor deliberately patchable: {unclassified:?}\n\
         A new `Passport` field must be classified. Add it to PROTECTED_PATCH_FIELDS \
         if a free-form patch must never reach it — which is the right answer for \
         anything the state machine, the publish pipeline, the registry or record \
         identity owns — or to DELIBERATELY_PATCHABLE, with the reason."
    );

    let both: Vec<&str> = DELIBERATELY_PATCHABLE
        .iter()
        .copied()
        .filter(|k| PROTECTED_PATCH_FIELDS.contains(k))
        .collect();

    assert!(
        both.is_empty(),
        "keys are in both lists, so the classification contradicts itself: {both:?}"
    );
}

/// Neither list may name a key `Passport` does not serialise.
///
/// The protected half is covered above. This catches the other direction: a
/// stale `DELIBERATELY_PATCHABLE` entry would silently shrink what
/// [`every_wire_key_is_classified`] checks, by classifying a key that no longer
/// exists while the real one goes unclassified.
#[test]
fn deliberately_patchable_keys_are_real_wire_keys() {
    let stale: Vec<&str> = DELIBERATELY_PATCHABLE
        .iter()
        .copied()
        .filter(|k| !PASSPORT_WIRE_KEYS.contains(k))
        .collect();

    assert!(
        stale.is_empty(),
        "DELIBERATELY_PATCHABLE names keys that are not `Passport` wire keys: {stale:?}"
    );
}
