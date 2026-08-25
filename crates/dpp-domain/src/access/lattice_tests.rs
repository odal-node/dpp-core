//! The Art. 77(2) lattice: neither audience contains the other, and each of
//! Annex XIII points 2, 3 and 4 reaches a different one.

use std::collections::HashMap;

use serde_json::json;

use crate::{Audience, Disclosure};

use super::filter::filter_by_audience;
use super::policy::{DocumentScope, ProductGroupAccessPolicy};
use super::tests::{battery_policy, filter_payload};

#[test]
fn the_annex_xiii_point_4_tier_is_withheld_through_the_real_catalog_policy() {
    // Not a hand-built policy: this reads product-groups/battery.json, so it fails if
    // a point-4 field is added to the type and its disclosure entry is
    // forgotten — which would publish measured, per-battery data to anyone
    // scanning the QR code.
    let policy = battery_policy();
    let data = json!({
        "gtin": "09506000134352",
        "dynamicPerformance": { "ratedCapacityAh": 92.0, "capacityFadePct": 8.0 },
        "batteryStatus": "repurposed",
        "usageHistory": { "chargeDischargeCycles": 412 },
    });

    let point_4 = ["dynamicPerformance", "batteryStatus", "usageHistory"];

    for audience in [Audience::Public, Audience::Authority] {
        let view = filter_payload(&data, &policy, audience);
        for key in point_4 {
            assert!(
                view.filtered_data.get(key).is_none(),
                "{audience:?} can see '{key}', which Annex XIII point 4 reserves \
                 to holders of a legitimate interest"
            );
        }
        assert!(
            view.filtered_data.get("gtin").is_some(),
            "{audience:?} lost a public field"
        );
    }

    let holder = filter_payload(&data, &policy, Audience::LegitimateInterest);
    for key in point_4 {
        assert!(
            holder.filtered_data.get(key).is_some(),
            "a legitimate-interest holder cannot see '{key}', which point 4 grants them"
        );
    }
}

#[test]
fn individual_item_data_is_withheld_from_authorities() {
    // The end-to-end consequence of the lattice, through the real filter: an
    // authority holds Annex XIII points 2 and 3, a legitimate-interest holder
    // holds 2 and 4. Neither sees everything.
    let mut field_disclosure = HashMap::new();
    field_disclosure.insert("dismantlingInfo".into(), Disclosure::Restricted);
    field_disclosure.insert("testReport".into(), Disclosure::Conformity);
    field_disclosure.insert("cycleHistory".into(), Disclosure::Individual);
    let policy = ProductGroupAccessPolicy {
        name: "lattice-test".into(),
        product_group: "battery".into(),
        field_disclosure,
        envelope_disclosure: HashMap::new(),
        default_disclosure: Disclosure::Public,
    };
    let data = json!({
        "productName": "Cell",
        "dismantlingInfo": "…",
        "testReport": "…",
        "cycleHistory": [1, 2, 3],
    });

    let authority = filter_payload(&data, &policy, Audience::Authority);
    assert!(authority.filtered_data.get("testReport").is_some());
    assert!(
        authority.filtered_data.get("cycleHistory").is_none(),
        "Art. 77(2)(b) does not grant authorities Annex XIII point 4"
    );

    let interest = filter_payload(&data, &policy, Audience::LegitimateInterest);
    assert!(interest.filtered_data.get("cycleHistory").is_some());
    assert!(
        interest.filtered_data.get("testReport").is_none(),
        "Art. 77(2)(c) does not grant legitimate interest Annex XIII point 3"
    );

    // Both see point 2; neither audience is a superset of the other.
    assert!(authority.filtered_data.get("dismantlingInfo").is_some());
    assert!(interest.filtered_data.get("dismantlingInfo").is_some());

    let public = filter_payload(&data, &policy, Audience::Public);
    assert_eq!(public.filtered_data.as_object().unwrap().len(), 1);
    assert!(public.filtered_data.get("productName").is_some());
}

/// Annex XIII points 2 and 3 land in different audiences, and the lattice is
/// not a ladder.
///
/// Point 2 reaches **both** non-public audiences; point 3 reaches authorities
/// **only**; point 4 reaches legitimate interest **only** and explicitly not
/// authorities. No integer ordering expresses that, which is why `Audience` is
/// not `Ord` — and why this reads the shipped catalog rather than a hand-built
/// policy, so a missing disclosure entry fails here instead of leaking.
#[test]
fn points_two_three_and_four_each_reach_a_different_audience() {
    let policy = battery_policy();
    let data = json!({
        "gtin": "09506000134352",
        "safetyMeasures": "Do not puncture",
        "testReportResults": "Report 42: pass",
        "batteryStatus": "original",
    });

    let public = filter_payload(&data, &policy, Audience::Public).filtered_data;
    for key in ["safetyMeasures", "testReportResults", "batteryStatus"] {
        assert!(
            public.get(key).is_none(),
            "the public view carries '{key}', which is not point 1 content"
        );
    }

    let authority = filter_payload(&data, &policy, Audience::Authority).filtered_data;
    assert!(authority.get("safetyMeasures").is_some(), "point 2");
    assert!(authority.get("testReportResults").is_some(), "point 3");
    assert!(
        authority.get("batteryStatus").is_none(),
        "point 4 is withheld from authorities — Art. 77(2)(b) does not reach it"
    );

    let holder = filter_payload(&data, &policy, Audience::LegitimateInterest).filtered_data;
    assert!(holder.get("safetyMeasures").is_some(), "point 2");
    assert!(holder.get("batteryStatus").is_some(), "point 4");
    assert!(
        holder.get("testReportResults").is_none(),
        "point 3 is authorities only — a legitimate interest does not reach test reports"
    );
}

/// The reproduction that started this: a battery passport's **envelope** must
/// not inherit a class from the battery *schema* because of a shared key name.
///
/// `Passport::applicable_instruments` once carried a `recordedAt`. The battery
/// schema declares its own `recordedAt` as individual-tier data (Annex XIII
/// point 4), and the filter classified by bare key name at every depth — so the
/// envelope field was stripped from every public battery projection, and the
/// document that came out no longer deserialised. That failure was loud only by
/// luck: the same collision on an optional field would have gone missing in
/// silence.
///
/// Asserted against the **real** battery policy, not a synthetic one, so it
/// holds against whatever the shipped schema actually declares.
#[test]
fn an_envelope_field_does_not_inherit_a_product_group_class_by_name() {
    let policy = battery_policy();
    assert_eq!(
        policy.disclosure_for_key("recordedAt", DocumentScope::ProductGroupData),
        Disclosure::Individual,
        "fixture assumption: the battery schema classifies recordedAt as individual"
    );

    let data = json!({
        "productGroup": "battery",
        "applicableInstruments": [
            { "instrument": "battery-reg-2023-1542", "recorded": "catalog",
              "recordedAt": "2026-01-01T00:00:00Z" }
        ],
        "productGroupData": {
            "productGroup": "battery",
            "recordedAt": "2026-01-01T00:00:00Z"
        }
    });

    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        let out = filter_by_audience(&data, &policy, audience);
        assert!(
            out.filtered_data["applicableInstruments"][0]
                .get("recordedAt")
                .is_some(),
            "{audience:?}: an envelope key must not be classified by the product \
             group's schema, got {}",
            out.filtered_data
        );
    }

    // The same name *inside* the payload is still the product group's to
    // classify, and still withheld from everyone but a legitimate-interest
    // holder. Both halves are the point: scoping the class must not disarm it.
    let public = filter_by_audience(&data, &policy, Audience::Public);
    assert!(
        public.filtered_data["productGroupData"]
            .get("recordedAt")
            .is_none(),
        "individual-tier payload data must stay withheld from the public"
    );
    assert!(
        public
            .redacted_fields
            .contains(&"productGroupData.recordedAt".to_owned())
    );
}
