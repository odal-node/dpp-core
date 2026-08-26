//! Which fields each audience is served by [`redact_passport`].
//!
//! The field-*classification* tripwires live beside the constants they police,
//! in `passport::redaction_tests`. These exercise the behaviour those lists
//! produce.

use crate::disclosure::Audience;
use crate::passport::tests::make_passport;
use crate::passport::{PASSPORT_PROOF_FIELDS, Passport};
use crate::product_group::{BatteryData, ProductGroup, ProductGroupData};

fn battery_passport_with_due_diligence() -> Passport {
    let mut p = make_passport();
    p.product_group = ProductGroup::Battery;
    // The fixture must declare a version whose schema actually contains the
    // fields it carries. `disassemblyInstructionsUrl` does not exist in battery
    // v1.0.0 — it arrives later — so a passport claiming v1.0.0 while carrying it
    // is not a shape any published record could have. Redaction is pinned to the
    // declared version, so an unrealistic fixture tests unrealistic rules.
    p.schema_version = "2.6.0".into();
    p.batch_id = Some("BATCH-42".into());
    p.jws_signature = Some("eyJhbGci.test.signature".into());
    p.product_group_data = Some(ProductGroupData::Battery(Box::new(BatteryData {
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        disassembly_instructions_url: Some("https://acme.example.com/disassembly".into()),
        ..crate::test_support::sample_battery_data()
    })));
    p
}

#[test]
fn redact_public_strips_batch_id_jws_and_retention() {
    let p = battery_passport_with_due_diligence();
    let view = crate::access::redact_passport(&p, Audience::Public).into_value();
    assert!(
        view.get("batchId").is_none(),
        "batchId must be stripped at Public"
    );
    assert!(
        view.get("jwsSignature").is_none(),
        "jwsSignature must be stripped at Public"
    );
    assert!(
        view.get("retentionLocked").is_none(),
        "retentionLocked must be stripped at Public"
    );
    assert!(
        view.get("productName").is_some(),
        "productName must survive"
    );
}

#[test]
fn redact_public_strips_gated_product_group_fields() {
    let p = battery_passport_with_due_diligence();
    let view = crate::access::redact_passport(&p, Audience::Public).into_value();
    let sd = &view["productGroupData"];
    assert!(
        sd.get("dueDiligenceUrl").is_some(),
        "dueDiligenceUrl is Annex XIII point 1(d) — publicly accessible"
    );
    assert!(
        sd.get("disassemblyInstructionsUrl").is_none(),
        "disassemblyInstructionsUrl is Annex XIII point 2(c) — withheld from the public"
    );
    assert!(
        sd.get("batteryChemistry").is_some(),
        "batteryChemistry is Public — must survive"
    );
    assert!(
        sd.get("co2ePerUnitKg").is_some(),
        "co2ePerUnitKg is Public — must survive"
    );
}

#[test]
fn redact_professional_exposes_gated_product_group_fields() {
    let p = battery_passport_with_due_diligence();
    let view = crate::access::redact_passport(&p, Audience::LegitimateInterest).into_value();
    let sd = &view["productGroupData"];
    assert!(
        sd.get("dueDiligenceUrl").is_some(),
        "Professional must see dueDiligenceUrl"
    );
    assert!(sd.get("disassemblyInstructionsUrl").is_some());
    // Still no JWS / retentionLocked at Professional
    assert!(view.get("jwsSignature").is_none());
    assert!(view.get("retentionLocked").is_none());
    // But batchId is visible
    assert!(view.get("batchId").is_some());
}

#[test]
fn redact_confidential_exposes_everything() {
    let p = battery_passport_with_due_diligence();
    let view = crate::access::redact_passport(&p, Audience::Authority).into_value();
    assert!(view.get("batchId").is_some());
    assert!(view.get("retentionLocked").is_some());
    // Not "everything": a proof is never part of any view, including an
    // authority's. `jwsSignature` covers the *full* payload, so attached to this
    // filtered body it would verify against nothing the reader was given.
    assert!(view.get("jwsSignature").is_none());
    let sd = &view["productGroupData"];
    assert!(sd.get("dueDiligenceUrl").is_some());
}

#[test]
fn redact_no_product_group_data_leaves_passport_fields() {
    let p = make_passport(); // no product_group_data, no batchId
    let view = crate::access::redact_passport(&p, Audience::Public).into_value();
    assert!(view.get("productName").is_some());
    assert!(view.get("productGroupData").is_none());
}

#[test]
fn redact_unknown_product_group_withholds_product_group_data_below_confidential() {
    let mut p = make_passport();
    // `Other` maps to catalog key "other", which is absent from the embedded
    // catalog — so there are no per-field disclosure classes to redact against.
    p.product_group = ProductGroup::Other("other".into());
    p.product_group_data = Some(
        ProductGroupData::other(serde_json::json!({ "secretField": "leak-me" }))
            .expect("an untagged payload has no typed variant"),
    );

    // No resolvable policy → the payload is reduced to its `productGroup` tag.
    // The tag is kept deliberately: a reader learns *that* there is product-group
    // data being withheld, rather than that there is none.
    let public = crate::access::redact_passport(&p, Audience::Public).into_value();
    assert_eq!(
        public["productGroupData"],
        serde_json::json!({ "productGroup": "other" }),
        "unresolvable policy must reduce to the tag, got: {}",
        public["productGroupData"]
    );
    assert!(
        !public.to_string().contains("leak-me"),
        "confidential product_group field leaked to a Public viewer"
    );

    // An authority is not an exception. Previously it received the full payload
    // for an unmodelled product group, on the reasoning that it may see every
    // class anyway. That confuses "entitled to every class" with "entitled to
    // fields whose class is unknown" — nothing here has been classified at all,
    // so there is no basis on which to hand any of it over.
    let authority = crate::access::redact_passport(&p, Audience::Authority).into_value();
    assert_eq!(
        authority["productGroupData"],
        serde_json::json!({ "productGroup": "other" }),
        "fail-closed must not have an audience-shaped hole in it"
    );
    assert!(
        !authority.to_string().contains("leak-me"),
        "an unclassified field reached an authority"
    );
}

#[test]
fn public_view_omits_every_non_public_passport_field() {
    // Regression: `redact` carried its own field list and omitted `lintResult`,
    // which the crypto layer's policy classified as Restricted. A public view
    // built through the domain path disclosed it. Both now read one table, and
    // this asserts the property rather than the three fields that were listed.
    use crate::disclosure::PASSPORT_FIELD_DISCLOSURE;

    let mut passport = make_passport();
    // Populate every non-public field so absence in the view proves redaction,
    // not that the field was simply unset.
    passport.batch_id = Some("BATCH-42".into());
    passport.jws_signature = Some("eyJhbGci.test.signature".into());
    passport.retention_locked = true;
    passport.lint_result = Some(crate::lint::LintResult {
        pack_version: "test".into(),
        findings: Vec::new(),
        assessed_at: chrono::Utc::now(),
    });

    let value = crate::access::redact_passport(&passport, Audience::Public).into_value();
    let obj = value.as_object().expect("view is an object");

    for (field, class) in PASSPORT_FIELD_DISCLOSURE {
        if !Audience::Public.may_see(*class) {
            assert!(
                !obj.contains_key(*field),
                "public view must not contain '{field}'"
            );
        }
    }

    // Guard against a vacuous pass: each field must actually be present for an
    // audience entitled to it, otherwise absence above proves nothing.
    let authority = crate::access::redact_passport(&passport, Audience::Authority).into_value();
    let authority = authority.as_object().expect("view is an object");
    for (field, class) in PASSPORT_FIELD_DISCLOSURE {
        // Proof fields are stripped for every audience, so they are legitimately
        // absent here and would make this guard fail for the wrong reason.
        if PASSPORT_PROOF_FIELDS.contains(field) {
            continue;
        }
        if Audience::Authority.may_see(*class) {
            assert!(
                authority.contains_key(*field),
                "'{field}' should be visible to an authority; absence above would be vacuous"
            );
        }
    }
}

// ── The leak this all exists to prevent ──────────────────────────────────────

/// Build a passport carrying every proof field populated, so absence in a view
/// proves redaction rather than an unset field.
fn passport_with_every_proof() -> Passport {
    let mut p = battery_passport_with_due_diligence();
    p.jws_signature = Some("eyJhbGci.ZnVsbC1wYXlsb2Fk.sig".into());
    p.public_jws_signature = Some("eyJhbGci.cHVibGljLXBheWxvYWQ.sig".into());
    p.disclosure_signatures.insert(
        "public+restricted+individual".into(),
        "eyJhbGci.cmVzdHJpY3RlZC1wYXlsb2Fk.sig".into(),
    );
    p.disclosure_signatures.insert(
        "public+restricted+conformity".into(),
        "eyJhbGci.Y29uZm9ybWl0eS1wYXlsb2Fk.sig".into(),
    );
    p.seal = Some(crate::seal::SealedEnvelope {
        format: crate::seal::SealFormat::Cades,
        seal_value: "cGtjczctYmxvYg==".into(),
        sealed_at: chrono::Utc::now(),
        placeholder: false,
        signing_cert_ref: None,
    });
    p
}

/// **No audience receives any proof field. Ever.**
///
/// The original defect in one assertion, run across all three audiences rather
/// than the one that happened to be noticed. `disclosureSignatures` is the
/// damaging one: those are *attached* compact JWS, so each embeds the full
/// redacted body for its own audience — serving the `public+restricted+individual`
/// entry to an anonymous reader hands over the restricted payload itself.
#[test]
fn no_audience_receives_any_proof_field() {
    let p = passport_with_every_proof();

    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        let view = crate::access::redact_passport(&p, audience).into_value();
        let obj = view.as_object().expect("view is an object");

        for proof in PASSPORT_PROOF_FIELDS {
            assert!(
                !obj.contains_key(*proof),
                "{audience:?} received the proof field '{proof}'"
            );
        }
    }
}

/// The payload a proof carries must not survive either, in any encoding.
///
/// Checking key absence alone would pass if a future change nested the proofs
/// somewhere else, or embedded them in a string. This asserts on the serialised
/// view, so a proof reintroduced under any name or depth still fails.
#[test]
fn no_proof_payload_survives_anywhere_in_a_served_view() {
    let p = passport_with_every_proof();

    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        let rendered = crate::access::redact_passport(&p, audience)
            .into_value()
            .to_string();

        for fragment in [
            "cmVzdHJpY3RlZC1wYXlsb2Fk", // base64 of the restricted payload
            "Y29uZm9ybWl0eS1wYXlsb2Fk", // base64 of the conformity payload
            "ZnVsbC1wYXlsb2Fk",         // base64 of the full payload
            "cGtjczctYmxvYg==",         // the seal blob
            "eyJhbGci",                 // any compact JWS header at all
        ] {
            assert!(
                !rendered.contains(fragment),
                "{audience:?}'s view contains '{fragment}' somewhere in the document"
            );
        }
    }
}

/// Redaction is pinned to the passport's own schema version, not today's map.
///
/// A passport's signatures are frozen over the redaction that produced them. If
/// filtering used whatever the catalog says now, a reclassification would break
/// verification for every already-published passport at once — the served body
/// and its proof would disagree for a reason no reader could distinguish from
/// tampering.
///
/// This asserts the mechanism directly: the same content under two declared
/// versions is filtered by two different policies.
#[test]
fn redaction_follows_the_declared_schema_version_not_the_current_one() {
    let mut modern = battery_passport_with_due_diligence();
    modern.schema_version = "2.6.0".into();
    let modern_view = crate::access::redact_passport(&modern, Audience::Public).into_value();

    let mut ancient = battery_passport_with_due_diligence();
    ancient.schema_version = "1.0.0".into();
    let ancient_view = crate::access::redact_passport(&ancient, Audience::Public).into_value();

    // v2.6.0 knows `disassemblyInstructionsUrl` and restricts it.
    assert!(
        modern_view["productGroupData"]
            .get("disassemblyInstructionsUrl")
            .is_none(),
        "v2.6.0 classifies this field as restricted"
    );

    // The two views are filtered by different rules — the version is load-bearing
    // rather than decorative. If this ever stops holding, redaction has silently
    // gone back to using one map for every version.
    assert_ne!(
        modern_view, ancient_view,
        "the declared schema version must change which policy applies"
    );
}

/// An unresolvable schema version fails closed rather than defaulting open.
///
/// This is the case a `catalog.get(key)` lookup could not distinguish: the
/// product group is perfectly well known, but *this version* of its schema is
/// not, so there are no per-field classes to filter against.
#[test]
fn a_known_product_group_at_an_unknown_version_still_fails_closed() {
    let mut p = battery_passport_with_due_diligence();
    p.schema_version = "99.0.0".into();

    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        let view = crate::access::redact_passport(&p, audience).into_value();
        assert_eq!(
            view["productGroupData"],
            serde_json::json!({ "productGroup": "battery" }),
            "{audience:?} got more than the tag for an unknown schema version"
        );
        assert!(
            !view.to_string().contains("disassembly"),
            "{audience:?} received product-group data with no policy to filter it"
        );
    }
}

/// Envelope classes still apply when the product-group policy cannot resolve.
///
/// Fail-closed on the payload must not become fail-*open* on the envelope: the
/// envelope's classes are version-independent, so an unresolvable product-group
/// version has no bearing on them.
#[test]
fn failing_closed_on_the_payload_does_not_disable_envelope_redaction() {
    let mut p = passport_with_every_proof();
    p.schema_version = "99.0.0".into();
    p.retention_locked = true;

    let view = crate::access::redact_passport(&p, Audience::Public).into_value();

    assert!(
        view.get("batchId").is_none(),
        "batchId is Restricted and must still be stripped"
    );
    assert!(
        view.get("retentionLocked").is_none(),
        "retentionLocked is Conformity and must still be stripped"
    );
    for proof in PASSPORT_PROOF_FIELDS {
        assert!(
            view.get(*proof).is_none(),
            "'{proof}' survived because the payload policy failed to resolve"
        );
    }
    assert!(
        view.get("productName").is_some(),
        "public envelope content must still be served"
    );
}
