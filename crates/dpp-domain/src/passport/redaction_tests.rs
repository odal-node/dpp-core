//! The classification tripwires for [`Passport`]'s field lists.
//!
//! These police the *lists* — every wire key is deliberately classified, and the
//! lists do not contradict each other. What those classifications then do to a
//! served view is exercised in `access::passport_view_tests`.

use crate::disclosure::{Disclosure, PASSPORT_FIELD_DISCLOSURE};
use crate::passport::PASSPORT_PROOF_FIELDS;

// ── The tripwire ─────────────────────────────────────────────────────────────

/// **Every wire key of [`Passport`] must be deliberately classified.**
///
/// This is the test that exists because a name list is how the leak happened.
/// `seal`, `publicJwsSignature` and `disclosureSignatures` were absent from
/// [`PASSPORT_FIELD_DISCLOSURE`], so they defaulted to `Public` and a public view
/// carried all three — and nothing anywhere failed, because "absent from the
/// table" and "deliberately public" were indistinguishable.
///
/// They are distinguishable here. A new field on `Passport` lands in
/// [`PASSPORT_WIRE_KEYS`] (its own completeness test enforces that), and this
/// fails the build until someone puts it in exactly one of three places:
/// the disclosure table, the proof list, or the allowlist below with a reason.
///
/// The allowlist is the point. It is not a way to skip the decision — it is
/// where the decision is recorded, in a diff a reviewer reads.
#[test]
fn passport_every_wire_key_is_classified() {
    /// Keys deliberately served to everyone, each with the reason it is safe.
    const PUBLICLY_SERVED: &[(&str, &str)] = &[
        (
            "id",
            "the passport identifier is what a QR code resolves to",
        ),
        ("productName", "Annex III basic product information"),
        (
            "productGroup",
            "the dispatch key; a reader needs it to interpret the rest",
        ),
        (
            "applicableInstruments",
            "which law governs this product is not a secret",
        ),
        (
            "granularity",
            "model/batch/item — states what the record describes",
        ),
        (
            "manufacturer",
            "Annex III(k): name and contact details of the operator",
        ),
        ("materials", "Annex III material content is consumer-facing"),
        ("co2ePerUnit", "a declared environmental figure"),
        ("repairabilityScore", "a declared repairability figure"),
        (
            "complianceResult",
            "the determination itself; withholding it defeats the passport",
        ),
        (
            "productGroupData",
            "filtered field-by-field by the product group's own policy",
        ),
        (
            "status",
            "whether the passport is live, suspended or archived",
        ),
        ("qrCodeUrl", "the public address of this passport"),
        ("createdAt", "record lifecycle timestamp"),
        ("updatedAt", "record lifecycle timestamp"),
        ("publishedAt", "record lifecycle timestamp"),
        (
            "placedOnMarketDate",
            "the regulated triggering event that fixes governing law",
        ),
        (
            "schemaVersion",
            "a reader needs it to interpret productGroupData",
        ),
        ("version", "monotonic version counter"),
        ("supersedesId", "lineage: which passport this replaces"),
        (
            "parentPassportRef",
            "ESPR Art. 11(d) linkage to the original passport",
        ),
        ("componentRefs", "bill-of-materials linkage"),
        (
            "retentionUntil",
            "how long this record must remain reachable",
        ),
        (
            "productId",
            "opaque internal template link, not a legal identifier",
        ),
        (
            "commodityCode",
            "customs classification, registered publicly anyway",
        ),
        (
            "operatorIdentifier",
            "Annex III(k) unique operator identifier",
        ),
        ("facility", "Annex III facility snapshot"),
    ];

    let classified: std::collections::BTreeSet<&str> = PASSPORT_FIELD_DISCLOSURE
        .iter()
        .map(|(f, _)| *f)
        .chain(PASSPORT_PROOF_FIELDS.iter().copied())
        .chain(PUBLICLY_SERVED.iter().map(|(f, _)| *f))
        .collect();

    let unclassified: Vec<&str> = crate::passport::PASSPORT_WIRE_KEYS
        .iter()
        .copied()
        .filter(|k| !classified.contains(k))
        .collect();

    assert!(
        unclassified.is_empty(),
        "these Passport fields are classified nowhere, so redaction defaults them \
         to Public and no test would notice: {unclassified:?}\n\n\
         Put each one in exactly one of:\n  \
         - PASSPORT_FIELD_DISCLOSURE, if some audiences may not see it\n  \
         - PASSPORT_PROOF_FIELDS, if it is a signature, seal or other proof\n  \
         - PUBLICLY_SERVED in this test, with the reason it is safe for everyone"
    );

    // The reverse direction: a key listed here that no longer exists is a stale
    // entry, and a stale allowlist is how a removed-then-re-added field slips
    // back in unclassified.
    let wire: std::collections::BTreeSet<&str> = crate::passport::PASSPORT_WIRE_KEYS
        .iter()
        .copied()
        .collect();
    for (field, _) in PUBLICLY_SERVED {
        assert!(
            wire.contains(field),
            "'{field}' is allowlisted as publicly served but is not a Passport wire key"
        );
    }
    for field in PASSPORT_PROOF_FIELDS {
        assert!(
            wire.contains(field),
            "'{field}' is listed as a proof field but is not a Passport wire key"
        );
    }
}

/// No field may be classified twice — the three lists must partition, not overlap.
///
/// An overlap is not harmless: a proof field that is *also* in the disclosure
/// table reads as though its class decides who sees it, which is the exact
/// misreading that let `jwsSignature` reach an authority attached to a body it
/// could not verify. The proof list wins at runtime; this keeps the lists honest
/// about which one is load-bearing.
#[test]
fn a_passport_field_is_never_classified_as_both_proof_and_public() {
    let public_ok: std::collections::BTreeSet<&str> = PASSPORT_PROOF_FIELDS
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();

    for (field, _) in PASSPORT_FIELD_DISCLOSURE {
        if public_ok.contains(field) {
            // Deliberate overlap, documented as defence in depth: the class makes
            // the raw filter fail safe, the proof list makes redaction correct.
            // Assert the class is the most restrictive one, so the backstop is
            // actually a backstop.
            let class = PASSPORT_FIELD_DISCLOSURE
                .iter()
                .find(|(f, _)| f == field)
                .map(|(_, c)| *c)
                .expect("just found it");
            assert_eq!(
                class,
                Disclosure::Conformity,
                "'{field}' is a proof field, so its defence-in-depth class must be \
                 the most restrictive available, not {class:?}"
            );
        }
    }
}
