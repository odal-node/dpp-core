//! Which passport fields a patch may never reach.

use super::protected_fields::PROTECTED_PATCH_FIELDS;
use crate::domain::passport::{FacilitySnapshot, PassportId, PassportRef};
use crate::domain::seal::{SealFormat, SealedEnvelope};
use crate::test_support::sample_passport;
use chrono::Utc;

/// Every protected key must be a key `Passport` actually serialises to.
///
/// The list is strings, and nothing else ties them to the struct. A renamed
/// field — or a typo — leaves an entry protecting a key that no longer
/// exists, and the field it was meant to protect silently becomes patchable.
/// That failure is invisible: the guard still runs, still finds nothing, and
/// still reports success.
///
/// The instance below sets every `skip_serializing_if` field, because a
/// field that is `None` does not appear in the JSON at all and would look
/// exactly like a stale entry.
#[test]
fn every_protected_key_is_a_real_passport_field() {
    let now = Utc::now();
    let reference = PassportRef {
        uri: "https://id.example/dpp/1".to_owned(),
        public_jws_hash: "0".repeat(64),
    };
    let mut passport = sample_passport();
    passport.public_jws_signature = Some("eyJ..a".to_owned());
    passport
        .disclosure_signatures
        .insert("public+restricted".to_owned(), "eyJ..b".to_owned());
    passport.retention_until = Some(now);
    passport.supersedes_id = Some(PassportId::new());
    passport.parent_passport_ref = Some(reference.clone());
    passport.component_refs = vec![reference];
    passport.operator_identifier = Some("DE123456789".to_owned());
    passport.facility = Some(FacilitySnapshot {
        scheme: "gln".to_owned(),
        value: "4012345000009".to_owned(),
        name: "Werk Nord".to_owned(),
        country: "DE".to_owned(),
        address: None,
    });
    passport.seal = Some(SealedEnvelope {
        format: SealFormat::Cades,
        seal_value: "MIIB".to_owned(),
        signing_cert_ref: None,
        sealed_at: now,
        placeholder: true,
    });

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
         Either the field was renamed and this list was not, or the instance above \
         does not populate it. Both mean the field is unprotected."
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
