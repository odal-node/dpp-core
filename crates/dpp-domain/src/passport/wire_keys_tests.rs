//! Every wire key the passport serialises, checked against the struct.

use super::PASSPORT_WIRE_KEYS;
use crate::test_support::fully_populated_passport;

/// `PASSPORT_WIRE_KEYS` must be exactly the key set a `Passport` emits.
///
/// Both directions matter. A missing key means a consumer checking itself
/// against this list would reject a query that is actually correct; an extra
/// key means the list blesses a string that addresses nothing, which is the
/// silent-NULL failure it exists to prevent.
#[test]
fn wire_keys_are_exactly_what_passport_emits() {
    let json = serde_json::to_value(fully_populated_passport()).expect("serialises");
    let emitted: std::collections::BTreeSet<String> = json
        .as_object()
        .expect("passport is a JSON object")
        .keys()
        .cloned()
        .collect();
    let declared: std::collections::BTreeSet<String> =
        PASSPORT_WIRE_KEYS.iter().map(|k| (*k).to_owned()).collect();

    let missing: Vec<&String> = emitted.difference(&declared).collect();
    let extra: Vec<&String> = declared.difference(&emitted).collect();

    assert!(
        missing.is_empty(),
        "Passport emits keys PASSPORT_WIRE_KEYS does not list: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "PASSPORT_WIRE_KEYS lists keys Passport does not emit: {extra:?} — either the field \
         was renamed, or `fully_populated_passport` does not populate it"
    );
}

/// The two derived lists must be subsets of the vocabulary, or they name
/// keys that address nothing.
#[test]
fn derived_lists_use_only_real_wire_keys() {
    use super::RETENTION_MUTABLE_FIELDS;
    use crate::ports::passport_repo::PROTECTED_PATCH_FIELDS;

    for (name, list) in [
        ("RETENTION_MUTABLE_FIELDS", RETENTION_MUTABLE_FIELDS),
        ("PROTECTED_PATCH_FIELDS", PROTECTED_PATCH_FIELDS),
    ] {
        let stale: Vec<&&str> = list
            .iter()
            .filter(|k| !PASSPORT_WIRE_KEYS.contains(k))
            .collect();
        assert!(
            stale.is_empty(),
            "{name} names keys that are not Passport wire keys: {stale:?}"
        );
    }
}
