//! Which passport fields stay mutable after the retention lock.

use super::RETENTION_MUTABLE_FIELDS;
use crate::test_support::sample_passport;

/// Every entry must be a key `Passport` actually serialises to.
///
/// The list is strings and nothing else ties it to the struct. A stale entry
/// names a key that no longer exists, so the field it was meant to keep
/// writable becomes frozen — and that failure only appears at runtime, on a
/// published record, when something tries to write it.
///
/// Only fields that always serialise are asserted here: the four proof
/// fields and `seal` are `skip_serializing_if`, so they are checked against
/// the populated instance below.
#[test]
fn every_mutable_key_is_a_real_passport_field() {
    let mut passport = sample_passport();
    passport.public_jws_signature = Some("eyJ..a".to_owned());
    passport.jws_signature = Some("eyJ..b".to_owned());
    passport
        .disclosure_signatures
        .insert("public+restricted".to_owned(), "eyJ..c".to_owned());
    passport.qr_code_url = Some("https://id.example/dpp/1".to_owned());
    passport.seal = Some(crate::seal::SealedEnvelope {
        format: crate::seal::SealFormat::Cades,
        seal_value: "MIIB".to_owned(),
        signing_cert_ref: None,
        sealed_at: chrono::Utc::now(),
        placeholder: true,
    });
    passport.lint_result = Some(crate::lint::LintResult {
        pack_version: "1.0.0".to_owned(),
        findings: Vec::new(),
        assessed_at: chrono::Utc::now(),
    });
    passport.published_at = Some(chrono::Utc::now());

    let json = serde_json::to_value(&passport).expect("passport serialises");
    let obj = json.as_object().expect("passport is a JSON object");

    let stale: Vec<&str> = RETENTION_MUTABLE_FIELDS
        .iter()
        .copied()
        .filter(|k| !obj.contains_key(*k))
        .collect();

    assert!(
        stale.is_empty(),
        "RETENTION_MUTABLE_FIELDS names keys `Passport` does not serialise to: {stale:?}"
    );
}

/// A field cannot be both frozen-by-retention-mutable and user-patchable in
/// the sense the repository guard means: these are different axes, but an
/// entry appearing in neither list and changing after publish is the case
/// that fails at runtime. This pins the overlap that does exist so a future
/// edit to either list has to think about the other.
#[test]
fn mutable_and_protected_overlap_is_deliberate() {
    use crate::ports::passport_repo::PROTECTED_PATCH_FIELDS;
    let both: Vec<&str> = RETENTION_MUTABLE_FIELDS
        .iter()
        .copied()
        .filter(|k| PROTECTED_PATCH_FIELDS.contains(k))
        .collect();
    // Every one of these is written by the system after publish and must
    // never be written by a user patch — that is why it is in both.
    assert_eq!(
        both,
        vec![
            "status",
            "publishedAt",
            "retentionLocked",
            "jwsSignature",
            "publicJwsSignature",
            "disclosureSignatures",
            "seal",
        ],
        "the overlap between system-writable-after-publish and \
         not-user-patchable changed; confirm the new shape is intended"
    );
}

/// The recorded legal basis is immutable, and it must be immutable in both
/// senses at once.
///
/// It is protected from patching, so no caller can edit which acts a
/// published passport was issued under; and it is *not* retention-mutable,
/// so it cannot change after the record is frozen either. A correction is a
/// new version that supersedes this one — the law at placing on the market
/// did not change, so a record claiming otherwise is a different record.
#[test]
fn the_recorded_legal_basis_can_never_be_edited_in_place() {
    assert!(
        crate::ports::passport_repo::PROTECTED_PATCH_FIELDS.contains(&"applicableInstruments"),
        "applicableInstruments must not be patchable"
    );
    assert!(
        !RETENTION_MUTABLE_FIELDS.contains(&"applicableInstruments"),
        "applicableInstruments must not change after retention lock"
    );
    // Granularity is set by the applicable delegated act, so it is fixed for
    // the same reason and by the same route.
    assert!(!RETENTION_MUTABLE_FIELDS.contains(&"granularity"));
}
