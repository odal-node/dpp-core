//! Seal capability negotiation, verification coherence, and format packaging.

use super::*;

#[test]
fn seal_format_serde_round_trips() {
    for fmt in [
        SealFormat::Jades,
        SealFormat::Pades,
        SealFormat::Cades,
        SealFormat::Xades,
    ] {
        let json = serde_json::to_string(&fmt).unwrap();
        let back: SealFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(fmt, back);
    }
}

/// A pass is not a pass is not a pass.
///
/// The whole reason the verdict is not a boolean: `TotalPassed` over a bare
/// signature check and `TotalPassed` over a qualified validation are different
/// claims, and only the second is one a compliance decision may rest on. If
/// these ever collapse, a self-signed development seal satisfies the same test
/// a qualified one does.
///
/// There is a third claim between them, which is the subject of
/// [`a_complete_ades_validation_is_not_yet_a_qualified_pass`].
#[test]
fn a_signature_check_is_not_a_qualified_pass() {
    let signature_only = SealVerification {
        indication: SealIndication::TotalPassed,
        checks: SealChecks::SignatureOnly,
        placeholder: false,
    };
    assert!(
        !signature_only.is_qualified_pass(),
        "a signature check says nothing about the certificate behind it"
    );

    let qualified = SealVerification {
        checks: SealChecks::QualifiedValidation,
        ..signature_only.clone()
    };
    assert!(qualified.is_qualified_pass());

    // And a placeholder never passes, however it is labelled.
    let placeholder = SealVerification {
        placeholder: true,
        ..qualified.clone()
    };
    assert!(!placeholder.is_qualified_pass());
}

/// A complete AdES validation is a pass, and still not a *qualified* pass.
///
/// The boundary this split exists to draw, asserted rather than described. An
/// AdES-complete check — certificate path, revocation, timestamp, signature —
/// is what any AdES library returns, and it establishes neither that the
/// certificate was qualified and QTSP-issued (Art. 32(1)(a)–(b) via Art. 40,
/// answerable only against a Trusted List) nor that a qualified creation device
/// was used (Art. 32(1)(f), via Annex III(j)).
///
/// Before the split there was one variant covering both, so this assertion
/// could not be written: the same verdict that meant "it verified" also meant
/// "a compliance claim may rest on it".
#[test]
fn a_complete_ades_validation_is_not_yet_a_qualified_pass() {
    let ades = SealVerification::passed(SealChecks::AdesValidation);

    assert!(
        ades.is_ades_pass(),
        "path, revocation and timestamp did check out"
    );
    assert!(
        !ades.is_qualified_pass(),
        "but nothing here consulted a Trusted List or looked for the Annex III(j) \
         creation-device indication"
    );

    let qualified = SealVerification::passed(SealChecks::QualifiedValidation);
    assert!(qualified.is_qualified_pass());
    assert!(
        qualified.is_ades_pass(),
        "the qualified check is strictly more than the AdES one, so it satisfies both"
    );
}

/// A signature-only pass is not an AdES pass either.
///
/// Guards the lower boundary of `is_ades_pass` the way the test above guards
/// its upper one. Without this, the new method could accept everything that
/// passed and the split would have moved the trap rather than closed it.
#[test]
fn a_signature_check_is_not_an_ades_pass() {
    assert!(!SealVerification::passed(SealChecks::SignatureOnly).is_ades_pass());
    assert!(!SealVerification::placeholder("nothing to validate").is_ades_pass());
}

/// Indeterminate is neither pass nor fail, and must not be read as either.
#[test]
fn indeterminate_is_not_a_pass() {
    let unresolved = SealVerification {
        indication: SealIndication::Indeterminate("revocation data unreachable".into()),
        checks: SealChecks::QualifiedValidation,
        placeholder: false,
    };
    assert!(!unresolved.is_qualified_pass());
    assert_ne!(unresolved.indication, SealIndication::TotalPassed);
}

#[test]
fn seal_mode_serde_round_trips() {
    for mode in [SealMode::ProviderSeal, SealMode::OperatorSeal] {
        let json = serde_json::to_string(&mode).unwrap();
        let back: SealMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }
}

#[test]
fn seal_envelope_serde_round_trips() {
    for envelope in SealEnvelope::ALL {
        let json = serde_json::to_string(envelope).unwrap();
        let back: SealEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(*envelope, back);
    }
}

/// Every packaging belongs to at least one format, and every format has one.
///
/// Guards the two ways the table and the enum can drift apart: a variant
/// added to [`SealEnvelope`] and never given to a format is unrequestable,
/// and a format whose row is empty cannot be asked for at all.
#[test]
fn every_format_and_packaging_is_reachable() {
    for format in SealFormat::ALL {
        assert!(
            !format.envelopes().is_empty(),
            "{format:?} defines no packaging, so no request for it is well-formed"
        );
    }
    for envelope in SealEnvelope::ALL {
        assert!(
            SealFormat::ALL.iter().any(|f| f.admits(*envelope)),
            "{envelope:?} belongs to no format, so nothing can ask for it"
        );
    }
}

/// The packagings are per-format, and the sets genuinely differ.
///
/// Transcribed from the CSC API's `signed_envelope_property` table. If these
/// ever collapse into one set, the distinction the table draws is gone and
/// `can_produce` is back to four independent membership checks.
#[test]
fn packagings_are_scoped_to_the_formats_that_define_them() {
    // `Enveloping` is XAdES's, and JAdES/CAdES do not define it.
    assert!(SealFormat::Xades.admits(SealEnvelope::Enveloping));
    assert!(!SealFormat::Jades.admits(SealEnvelope::Enveloping));
    assert!(!SealFormat::Cades.admits(SealEnvelope::Enveloping));

    // PAdES is disjoint from every other format.
    assert!(SealFormat::Pades.admits(SealEnvelope::Certification));
    assert!(!SealFormat::Pades.admits(SealEnvelope::Detached));
    assert!(!SealFormat::Jades.admits(SealEnvelope::Certification));

    // `Parallel` — two parties sealing the same passport — is JAdES and
    // CAdES only, and was the packaging missing entirely before this table.
    assert!(SealFormat::Jades.admits(SealEnvelope::Parallel));
    assert!(SealFormat::Cades.admits(SealEnvelope::Parallel));
    assert!(!SealFormat::Xades.admits(SealEnvelope::Parallel));
}

/// A defaulted PAdES request is unsatisfiable, and that is on purpose.
///
/// `envelope` defaults to [`SealEnvelope::Detached`] for every format,
/// because a request built from a payload hash means the caller already
/// holds the bytes. PAdES defines only `Certification` and `Revision`, so a
/// PAdES request that takes the default asks for a shape PAdES does not
/// have, and **no** advertisement can satisfy it.
///
/// The refusal is correct — a detached PAdES signature is not a thing — but
/// it is reached by a default rather than by anything the caller wrote, so
/// it is pinned here rather than left to be discovered. A format-aware
/// default would need a hand-written `Deserialize`, which is a large amount
/// of machinery for a format this crate does not otherwise use.
#[test]
fn a_defaulted_pades_request_cannot_be_satisfied() {
    // Everything advertised, including both packagings PAdES defines.
    let everything = SealCapabilities {
        supported_formats: SealFormat::ALL.to_vec(),
        supported_modes: SealMode::ALL.to_vec(),
        supported_levels: SealConformanceLevel::ALL.to_vec(),
        supported_envelopes: SealEnvelope::ALL.to_vec(),
    };

    let wire = r#"{
        "payloadHash": "abababababababababababababababababababababababababababababababab",
        "mode": "provider_seal",
        "keyRef": { "qtspId": "q", "credentialId": "c" },
        "sigFormat": "PADES"
    }"#;
    let defaulted: SealRequest = serde_json::from_str(wire).expect("defaults fill the rest");

    assert_eq!(
        defaulted.envelope,
        SealEnvelope::Detached,
        "the default is format-blind, which is the premise of this test"
    );
    assert!(
        !everything.can_produce(&defaulted),
        "PAdES does not define Detached, so nothing can produce this request"
    );

    // Naming a packaging PAdES actually defines is satisfiable, so the
    // refusal is about the default and not about PAdES.
    let named = SealRequest {
        envelope: SealEnvelope::Certification,
        ..defaulted
    };
    assert!(everything.can_produce(&named));
}

/// A pair no format defines is refused however generous the advertisement.
///
/// The defect this pins: an adapter listing `Jades` and `Enveloping`
/// separately reads as offering their combination, which the protocol
/// carrying the request has no way to express.
#[test]
fn can_produce_refuses_a_pair_no_format_defines() {
    let capabilities = SealCapabilities {
        supported_formats: vec![SealFormat::Jades, SealFormat::Xades],
        supported_modes: vec![SealMode::ProviderSeal],
        supported_levels: vec![SealConformanceLevel::BaselineLt],
        supported_envelopes: vec![SealEnvelope::Enveloping, SealEnvelope::Detached],
    };
    let request = |sig_format: SealFormat, envelope: SealEnvelope| SealRequest {
        payload_hash: "ab".repeat(32),
        mode: SealMode::ProviderSeal,
        key_ref: SealCredentialRef {
            qtsp_id: "q".into(),
            credential_id: "c".into(),
        },
        sig_format,
        conformance_level: SealConformanceLevel::BaselineLt,
        envelope,
    };

    // Every axis is advertised, and the pair is still not one that exists.
    assert!(!capabilities.can_produce(&request(SealFormat::Jades, SealEnvelope::Enveloping)));

    // The same packaging under the format that defines it is fine, so the
    // refusal is about the pair and not about `Enveloping`.
    assert!(capabilities.can_produce(&request(SealFormat::Xades, SealEnvelope::Enveloping)));
    assert!(capabilities.can_produce(&request(SealFormat::Jades, SealEnvelope::Detached)));
}

/// A stored envelope written before `conformanceLevel` existed still reads.
///
/// `SealedEnvelope` is persisted onto `Passport::seal` and travels on the wire,
/// so this is a compatibility surface with a hard property: **a published
/// passport is retention-locked, and its seal can never be rewritten.** Every
/// envelope sealed before this field was added is therefore permanent, and a
/// deserialiser that required the field would make those passports unreadable —
/// not degraded, unreadable, with no repair available.
#[test]
fn an_envelope_without_a_level_still_deserialises() {
    let legacy = serde_json::json!({
        "format": "CADES",
        "sealValue": "MIIB",
        "sealedAt": "2026-08-14T00:00:00Z",
        "placeholder": false,
    });

    let env: SealedEnvelope = serde_json::from_value(legacy).expect("legacy envelope reads");
    assert_eq!(
        env.conformance_level, None,
        "an absent level must read as `not recorded`, never as a default"
    );
}

/// The level survives the round trip, at every level and when absent.
///
/// Absent is included deliberately: `skip_serializing_if` means a `None` must
/// vanish from the JSON rather than serialising as `null`, and a passport whose
/// seal grew a `"conformanceLevel": null` key would be a content change on a
/// retention-locked document.
#[test]
fn the_conformance_level_round_trips_including_absent() {
    let envelope = |level| SealedEnvelope {
        format: SealFormat::Cades,
        seal_value: "MIIB".to_owned(),
        signing_cert_ref: None,
        conformance_level: level,
        sealed_at: chrono::Utc::now(),
        placeholder: false,
    };

    for level in SealConformanceLevel::ALL {
        let json = serde_json::to_value(envelope(Some(*level))).unwrap();
        assert_eq!(json["conformanceLevel"], serde_json::json!(level));
        let back: SealedEnvelope = serde_json::from_value(json).unwrap();
        assert_eq!(back.conformance_level, Some(*level));
    }

    let json = serde_json::to_value(envelope(None)).unwrap();
    assert!(
        json.get("conformanceLevel").is_none(),
        "an unrecorded level must be absent from the JSON, not null: {json}"
    );
}
