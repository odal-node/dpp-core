//! The publish gate on mandatory battery content: which fields block a publish,
//! for which battery types, and what the preview promises.

use super::*;
use crate::domain::product_group::{BatteryData, ProductGroup, ProductGroupData};
use crate::domain::status::PassportStatus;

// Blocking a publish is a serious act, so these cover the boundaries rather
// than the happy path: which category owes what, when the gate does *not* fire,
// and the one hole that is deliberate.

/// Every field the EV category makes mandatory, so a test can remove exactly
/// one and attribute the refusal to it.
fn publishable_battery(battery_type: crate::domain::product_group::BatteryType) -> Passport {
    use crate::domain::product_group::{
        BatteryStatus, DynamicPerformance, HazardousSubstance, MaterialComposition, StateOfHealth,
        TemperatureRange,
    };
    let range = TemperatureRange {
        min_c: -20.0,
        max_c: 60.0,
    };
    let mat = || {
        Some(vec![MaterialComposition {
            name: "LiFePO4".into(),
            weight_pct: 100.0,
            cas_number: None,
        }])
    };
    let data = BatteryData {
        battery_type,
        battery_weight_kg: Some(400.0),
        hazardous_substances: Some(vec![HazardousSubstance {
            name: "Nickel sulfate".into(),
            cas_number: None,
            concentration_pct: None,
        }]),
        usable_extinguishing_agent: Some("Class D dry powder".into()),
        critical_raw_materials: Some(vec![]),
        recycled_content_cobalt_pct: Some(4.0),
        recycled_content_lithium_pct: Some(4.0),
        recycled_content_nickel_pct: Some(4.0),
        recycled_content_lead_pct: Some(0.0),
        renewable_content_pct: Some(10.0),
        minimal_voltage_v: Some(2.5),
        maximum_voltage_v: Some(4.2),
        original_power_capability_w: Some(150_000.0),
        power_limit_min_w: Some(1_000.0),
        power_limit_max_w: Some(180_000.0),
        expected_lifetime_cycles: Some(3000),
        expected_lifetime_reference_test: Some("IEC 62660-1:2018".into()),
        capacity_threshold_for_exhaustion_pct: Some(80.0),
        not_in_use_temperature_range: Some(range),
        not_in_use_temperature_reference_test: Some("IEC 62660-1:2018".into()),
        initial_round_trip_efficiency_pct: Some(96.0),
        round_trip_efficiency_at_half_cycle_life_pct: Some(92.0),
        internal_cell_resistance_mohm: Some(1.2),
        internal_pack_resistance_mohm: Some(30.0),
        cycle_life_test_c_rate: Some(1.0),
        marking_information: Some("Separate collection symbol".into()),
        eu_declaration_of_conformity: Some("DoC-2027-0001".into()),
        waste_battery_information: Some("https://example.invalid/waste".into()),
        cathode_material: mat(),
        anode_material: mat(),
        electrolyte_material: mat(),
        component_part_numbers: Some(vec!["PN-1".into()]),
        spare_parts_contacts: Some("spares@example.invalid".into()),
        disassembly_instructions_url: Some("https://example.invalid/disassembly".into()),
        safety_measures: Some("Do not puncture".into()),
        test_report_results: Some("Report 42: pass".into()),
        dynamic_performance: Some(Box::new(DynamicPerformance::default())),
        state_of_health: Some(Box::new(StateOfHealth::ElectricVehicle { soce_pct: 99.0 })),
        battery_status: Some(BatteryStatus::Original),
        // Guidance data points 1, 7, 8 and 9 — mandatory for every covered
        // category, and unrepresentable until v2.6.0 declared them.
        battery_passport_number: Some("URN:UUID:6F1C9D2E-0000-4000-8000-000000000000".into()),
        battery_model_id: Some("LFP-64-A".into()),
        manufacturing_place: Some("PL:Wrocław".into()),
        manufacturing_date: Some(
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 3, 1, 0, 0, 0).unwrap(),
        ),
        ..crate::test_support::sample_battery_data()
    };
    Passport {
        product_group: ProductGroup::Battery,
        product_group_data: Some(ProductGroupData::Battery(Box::new(data))),
        ..crate::test_support::sample_passport()
    }
}

fn battery_field(p: &mut Passport, mutate: impl FnOnce(&mut BatteryData)) {
    if let Some(ProductGroupData::Battery(b)) = p.product_group_data.as_mut() {
        mutate(b);
    }
}

#[test]
fn a_complete_ev_battery_publishes() {
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    p.transition_to(PassportStatus::Published)
        .expect("a passport carrying every mandatory field must publish");
    assert!(p.retention_locked);
    assert!(p.published_at.is_some());
}

#[test]
fn a_missing_mandatory_field_blocks_names_it_and_leaves_no_lock() {
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);

    let err = p
        .transition_to(PassportStatus::Published)
        .expect_err("Annex VI Part A point 9 is mandatory for every covered category");
    let msg = err.to_string();
    assert!(
        msg.contains("usableExtinguishingAgent"),
        "the refusal must name the field: {msg}"
    );
    // A refused publish must leave nothing behind. Retention lock is permanent,
    // so setting it on a failed attempt would make the passport unrepairable.
    assert!(!p.retention_locked, "a refused publish must not lock");
    assert!(p.published_at.is_none());
    assert_eq!(p.status, PassportStatus::Draft);
}

#[test]
fn the_preview_gives_the_same_answer_as_the_attempt_and_changes_nothing() {
    // The gate is reachable without attempting the transition, and asking is
    // not declining: the preview returns the refusal verbatim, and the passport
    // is untouched either way.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);

    let previewed = p
        .check_mandatory_content()
        .expect_err("the preview must refuse what the transition refuses");

    // Asking left no trace.
    assert_eq!(p.status, PassportStatus::Draft);
    assert!(!p.retention_locked);
    assert!(p.published_at.is_none());

    let attempted = p
        .transition_to(PassportStatus::Published)
        .expect_err("the transition must still refuse");
    assert_eq!(
        previewed.to_string(),
        attempted.to_string(),
        "a preview that does not render identically to the refusal is a second \
         opinion, and the two would drift"
    );
}

#[test]
fn the_preview_passes_for_a_passport_that_publishes() {
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    p.check_mandatory_content()
        .expect("a complete passport must preview clean");
    p.transition_to(PassportStatus::Published)
        .expect("and must then actually publish");
}

/// The four identity data points the guidance marks mandatory for every
/// covered category each block a publish on their own.
///
/// These were absent from the requirements table *and* from every battery
/// schema property, so a passport could be published carrying none of
/// them: no unique identifier, no model identification, and no record of where
/// or when the battery was made. The schema could not even store them —
/// `additionalProperties: false` rejected all four — so this test is the guard
/// on both halves of that defect at once. It fails if either the requirements
/// row or the schema property is removed.
#[test]
fn each_identity_data_point_blocks_publish_on_its_own() {
    for (name, clear) in [
        (
            "batteryPassportNumber",
            (|b: &mut BatteryData| b.battery_passport_number = None) as fn(&mut BatteryData),
        ),
        ("batteryModelId", |b: &mut BatteryData| {
            b.battery_model_id = None;
        }),
        ("manufacturingPlace", |b: &mut BatteryData| {
            b.manufacturing_place = None;
        }),
        ("manufacturingDate", |b: &mut BatteryData| {
            b.manufacturing_date = None;
        }),
    ] {
        let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
        battery_field(&mut p, clear);

        let err = match p.transition_to(PassportStatus::Published) {
            Err(e) => e,
            Ok(()) => panic!(
                "{name} is mandatory for every covered category, but the publish was allowed"
            ),
        };
        let msg = format!("{err:?}");
        assert!(msg.contains(name), "the refusal must name {name}: {msg}");
        assert!(!p.retention_locked, "a refused publish must not lock");
        assert_eq!(p.status, PassportStatus::Draft);
    }
}

#[test]
fn every_missing_field_is_reported_at_once() {
    // One-at-a-time reporting turns a single fix into N publish attempts.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| {
        b.usable_extinguishing_agent = None;
        b.marking_information = None;
    });
    let msg = p
        .transition_to(PassportStatus::Published)
        .unwrap_err()
        .to_string();
    assert!(msg.contains("usableExtinguishingAgent"), "{msg}");
    assert!(msg.contains("markingInformation"), "{msg}");
}

#[test]
fn the_ev_only_field_is_demanded_of_ev_and_not_of_lmt() {
    // Annex XIII point 1(k): mandatory for EV, "not to be filled/displayed" for
    // LMT and industrial. The sharpest per-category split in the guidance and
    // the one most easily flattened by a careless edit.
    let mut ev = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut ev, |b| b.capacity_threshold_for_exhaustion_pct = None);
    assert!(
        ev.transition_to(PassportStatus::Published).is_err(),
        "1(k) is mandatory for EV"
    );

    let mut lmt = publishable_battery(crate::domain::product_group::BatteryType::Lmt);
    battery_field(&mut lmt, |b| b.capacity_threshold_for_exhaustion_pct = None);
    lmt.transition_to(PassportStatus::Published)
        .expect("1(k) is not applicable to LMT, so its absence cannot block");
}

#[test]
fn industrial_may_publish_without_a_cycle_lifetime() {
    // Point 1(j) reaches industrial batteries only "where lifetime can be
    // expressed in cycles". That carve-out is why the field became optional;
    // the gate must not quietly reintroduce the requirement.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Industrial);
    battery_field(&mut p, |b| {
        b.expected_lifetime_cycles = None;
        b.expected_lifetime_reference_test = None;
        b.cycle_life_test_c_rate = None;
        b.initial_round_trip_efficiency_pct = None;
        b.round_trip_efficiency_at_half_cycle_life_pct = None;
    });
    p.transition_to(PassportStatus::Published)
        .expect("every field removed here is conditional for industrial batteries");
}

#[test]
fn portable_and_sli_are_ungated_and_that_is_deliberate() {
    // The guidance covers EV, LMT and industrial only. Blocking a portable
    // battery would invent a requirement the source declines to state — the
    // defect class this project keeps catching in other people's work. A real
    // hole, held open on purpose until a source covering them exists.
    for t in [
        crate::domain::product_group::BatteryType::Portable,
        crate::domain::product_group::BatteryType::Sli,
    ] {
        let mut p = publishable_battery(t);
        battery_field(&mut p, |b| {
            b.usable_extinguishing_agent = None;
            b.marking_information = None;
            b.eu_declaration_of_conformity = None;
        });
        p.transition_to(PassportStatus::Published)
            .expect("no source covers this category, so nothing is gated");
    }
}

#[test]
fn a_republish_is_not_re_gated() {
    // `transition_to` also runs on Suspended → Published. Gating a republish
    // would let a later change to the requirements table strand a passport
    // published lawfully under the earlier one — and retention lock means the
    // operator could not repair it. Content is judged once, at first publish.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Suspended).unwrap();

    // Stand in for the table tightening under an already-published record.
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);

    p.transition_to(PassportStatus::Published)
        .expect("a republish must not be blocked by a rule that arrived after issuance");
}

#[test]
fn a_battery_passport_without_product_group_data_cannot_publish() {
    let mut p = Passport {
        product_group: ProductGroup::Battery,
        product_group_data: None,
        ..crate::test_support::sample_passport()
    };
    let err = p.transition_to(PassportStatus::Published).unwrap_err();
    assert!(err.to_string().contains("productGroupData"), "{err}");
}

#[test]
fn a_non_battery_product_group_is_untouched_by_the_gate() {
    let mut p = crate::test_support::sample_passport();
    p.product_group = ProductGroup::Textile;
    p.product_group_data = None;
    p.transition_to(PassportStatus::Published)
        .expect("no requirements table exists for textile, so the gate is inert");
}

#[test]
fn a_non_publish_transition_is_not_gated() {
    // Only publish is judged. An incomplete draft must stay movable, or an
    // operator cannot abandon one they decided not to finish.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);
    p.transition_to(PassportStatus::Archived)
        .expect("archiving an incomplete draft is not a compliance claim");
}
