//! The personal-data check: marks read from the schemas, statements required for
//! marked fields with a value, and the consent-only condition of the acts that
//! set one.

use std::collections::BTreeSet;

use chrono::Utc;

use super::personal_data::{PERSONAL_DATA_MARK, check_personal_data, personal_data_marks};
use super::validate_passport;
use crate::field_error::FieldError;
use crate::instrument::InstrumentRef;
use crate::passport::Passport;
use crate::personal_data::{HeldOutside, LawfulBasis, PersonalDataRecordId, PersonalDataStatement};
use crate::product_group::{
    BatteryData, EnvironmentalReading, FibreEntry, ProductGroup, ProductGroupData, TextileData,
    UsageHistory,
};
use crate::schemas::VersionedSchemaRegistry;

fn set(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|p| (*p).to_owned()).collect()
}

fn battery_with_usage(usage: UsageHistory) -> Passport {
    Passport {
        product_group: ProductGroup::Battery,
        applicable_instruments: vec![InstrumentRef::from_catalog("battery-reg-2023-1542")],
        product_group_data: Some(ProductGroupData::Battery(Box::new(BatteryData {
            usage_history: Some(Box::new(usage)),
            ..crate::test_support::sample_battery_data()
        }))),
        schema_version: "2.8.0".into(),
        ..crate::test_support::sample_passport()
    }
}

fn accident() -> UsageHistory {
    UsageHistory {
        negative_events: Some(vec!["collision; pack housing deformed".into()]),
        ..UsageHistory::default()
    }
}

fn textile_with_repairs(repairs: Option<&str>) -> Passport {
    Passport {
        product_group: ProductGroup::Textile,
        product_group_data: Some(ProductGroupData::Textile(Box::new(TextileData {
            repair_history_url: repairs.map(str::to_owned),
            fibre_composition: vec![FibreEntry {
                fibre: "cotton".into(),
                pct: 100.0,
                country_of_origin: None,
            }],
            ..crate::test_support::sample_textile_data()
        }))),
        schema_version: "1.4.0".into(),
        ..crate::test_support::sample_passport()
    }
}

fn held_outside(basis: LawfulBasis) -> PersonalDataStatement {
    PersonalDataStatement::HeldOutside(HeldOutside::new(basis, PersonalDataRecordId::new("rec-1")))
}

fn fields(errors: &[FieldError]) -> Vec<&str> {
    errors.iter().map(|e| e.field.as_str()).collect()
}

// ── Marks ────────────────────────────────────────────────────────────────

#[test]
fn battery_marks_the_two_free_text_fields_of_the_individual_tier() {
    assert_eq!(
        personal_data_marks("battery", "2.8.0").unwrap(),
        set(&[
            "usageHistory.negativeEvents",
            "usageHistory.operatingConditions.note",
        ]),
    );
}

#[test]
fn textile_marks_the_repair_log_from_v1_4_0_and_not_before() {
    assert_eq!(
        personal_data_marks("textile", "1.4.0").unwrap(),
        set(&["repairHistoryUrl"])
    );
    assert!(personal_data_marks("textile", "1.3.0").unwrap().is_empty());
}

#[test]
fn a_schema_the_registry_does_not_hold_has_no_marks_to_read() {
    assert_eq!(personal_data_marks("battery", "9.9.9"), None);
    assert_eq!(personal_data_marks("plastics", "1.0.0"), None);
    assert_eq!(personal_data_marks("battery", "not-semver"), None);
}

/// Every mark in every shipped schema is one the walk reaches, and is `true`.
///
/// The walk follows `properties`, `items`, the combinators and local `$ref`,
/// because those are the places a mark has a fixed path. A mark written
/// anywhere else would be collected by nothing and enforced by nothing, so it
/// fails here instead of passing silently.
#[test]
fn every_mark_in_every_shipped_schema_is_collected() {
    fn count(value: &serde_json::Value, found: &mut usize) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(mark) = map.get(PERSONAL_DATA_MARK) {
                    assert_eq!(
                        mark,
                        &serde_json::Value::Bool(true),
                        "{PERSONAL_DATA_MARK} is a flag, and false is written by leaving it out"
                    );
                    *found += 1;
                }
                map.values().for_each(|v| count(v, found));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| count(v, found)),
            _ => {}
        }
    }

    let registry = VersionedSchemaRegistry::new();
    for (group, version) in registry.list() {
        let root: serde_json::Value =
            serde_json::from_str(registry.get(group, version).unwrap()).unwrap();
        let mut written = 0;
        count(&root, &mut written);
        let collected = personal_data_marks(group, &version.to_string())
            .unwrap()
            .len();
        assert_eq!(
            written, collected,
            "{} v{}: {written} mark(s) written, {collected} collected — a mark sits where \
             no fixed path reaches it",
            group, version
        );
    }
}

// ── Statements required ──────────────────────────────────────────────────

#[test]
fn a_marked_field_with_a_value_and_no_statement_is_refused() {
    let errors = check_personal_data(&battery_with_usage(accident()));
    assert_eq!(
        fields(&errors),
        ["/productGroupData/usageHistory/negativeEvents"]
    );
    assert!(errors[0].message.contains("personalData must state"));
}

#[test]
fn either_statement_answers_the_refusal() {
    for statement in [
        PersonalDataStatement::NothingHeld,
        held_outside(LawfulBasis::Consent),
    ] {
        let mut passport = battery_with_usage(accident());
        passport
            .personal_data
            .insert("usageHistory.negativeEvents".into(), statement);
        assert_eq!(check_personal_data(&passport), Vec::new());
    }
}

#[test]
fn a_marked_field_inside_an_array_is_found_in_every_element() {
    let reading = |note: Option<&str>| EnvironmentalReading {
        recorded_at: Utc::now(),
        temperature_c: Some(21.0),
        note: note.map(str::to_owned),
    };
    let usage = UsageHistory {
        operating_conditions: Some(vec![reading(None), reading(Some("parked in a garage"))]),
        ..UsageHistory::default()
    };
    assert_eq!(
        fields(&check_personal_data(&battery_with_usage(usage))),
        ["/productGroupData/usageHistory/operatingConditions/note"]
    );
}

#[test]
fn an_empty_or_blank_marked_field_needs_no_statement() {
    for events in [
        None,
        Some(vec![]),
        Some(vec!["  ".to_owned(), String::new()]),
    ] {
        let usage = UsageHistory {
            negative_events: events,
            ..UsageHistory::default()
        };
        assert_eq!(check_personal_data(&battery_with_usage(usage)), Vec::new());
    }
}

/// Numbers the Regulation requires are not marked, and a statement is never
/// asked of them: storing them is the obligation.
#[test]
fn required_measurements_are_not_asked_about() {
    let usage = UsageHistory {
        charge_discharge_cycles: Some(412),
        operating_conditions: Some(vec![EnvironmentalReading {
            recorded_at: Utc::now(),
            temperature_c: Some(35.0),
            note: None,
        }]),
        ..UsageHistory::default()
    };
    assert_eq!(check_personal_data(&battery_with_usage(usage)), Vec::new());
}

// ── Statements refused ───────────────────────────────────────────────────

#[test]
fn a_statement_about_an_unmarked_field_is_refused() {
    let mut passport = battery_with_usage(UsageHistory::default());
    passport
        .personal_data
        .insert("safetyMeasures".into(), PersonalDataStatement::NothingHeld);
    let errors = check_personal_data(&passport);
    assert_eq!(fields(&errors), ["/personalData/safetyMeasures"]);
}

#[test]
fn a_record_held_outside_must_be_named() {
    let mut passport = battery_with_usage(accident());
    passport.personal_data.insert(
        "usageHistory.negativeEvents".into(),
        PersonalDataStatement::HeldOutside(HeldOutside::new(
            LawfulBasis::Consent,
            PersonalDataRecordId::new(" "),
        )),
    );
    assert_eq!(
        fields(&check_personal_data(&passport)),
        ["/personalData/usageHistory.negativeEvents/record"]
    );
}

// ── Consent-only acts ────────────────────────────────────────────────────

/// The Batteries Regulation sets no condition of its own, so any GDPR basis is
/// the controller's to name.
#[test]
fn a_battery_record_may_be_held_on_any_basis() {
    let mut passport = battery_with_usage(accident());
    passport.personal_data.insert(
        "usageHistory.negativeEvents".into(),
        held_outside(LawfulBasis::LegalObligation),
    );
    assert_eq!(check_personal_data(&passport), Vec::new());
}

#[test]
fn an_espr_passport_holds_customer_personal_data_only_with_consent() {
    let mut passport = textile_with_repairs(Some("seam repaired"));
    passport.personal_data.insert(
        "repairHistoryUrl".into(),
        held_outside(LawfulBasis::Contract),
    );
    let errors = check_personal_data(&passport);
    assert_eq!(
        fields(&errors),
        ["/personalData/repairHistoryUrl/lawfulBasis"]
    );
    assert!(
        errors[0].message.contains("'contract'")
            && errors[0]
                .message
                .contains("Regulation (EU) 2024/1781 Art. 10(1)(e)"),
        "the refusal names the basis given and the provision it fails: {}",
        errors[0].message
    );

    passport.personal_data.insert(
        "repairHistoryUrl".into(),
        held_outside(LawfulBasis::Consent),
    );
    assert_eq!(check_personal_data(&passport), Vec::new());
}

/// A passport that has not recorded its acts is held to the acts its product
/// group is bound to, not excused from them.
#[test]
fn with_no_recorded_acts_the_product_groups_acts_govern() {
    let mut passport = textile_with_repairs(Some("seam repaired"));
    passport.applicable_instruments.clear();
    passport.personal_data.insert(
        "repairHistoryUrl".into(),
        held_outside(LawfulBasis::Contract),
    );
    assert_eq!(
        fields(&check_personal_data(&passport)),
        ["/personalData/repairHistoryUrl/lawfulBasis"]
    );
}

/// What a passport records is what governs it. One recording only an act with
/// no condition is held to none, even in a product group ESPR binds.
#[test]
fn the_recorded_acts_govern_when_there_are_any() {
    let mut passport = textile_with_repairs(Some("seam repaired"));
    passport.applicable_instruments = vec![InstrumentRef::from_catalog("battery-reg-2023-1542")];
    passport.personal_data.insert(
        "repairHistoryUrl".into(),
        held_outside(LawfulBasis::Contract),
    );
    assert_eq!(check_personal_data(&passport), Vec::new());
}

// ── Through validate_passport ────────────────────────────────────────────

#[test]
fn validate_passport_refuses_a_marked_field_without_a_statement() {
    let passport = textile_with_repairs(Some("https://repairs.example/item/1"));
    let err = validate_passport(&passport).expect_err("an unstated repair log is refused");
    let crate::error::DppError::Validation(ve) = err else {
        panic!("expected a validation error, got {err}");
    };
    assert!(
        ve.errors
            .iter()
            .any(|e| e.field == "/productGroupData/repairHistoryUrl"),
        "{:?}",
        ve.errors
    );
}

#[test]
fn validate_passport_accepts_the_same_passport_with_its_statement() {
    let mut passport = textile_with_repairs(Some("https://repairs.example/item/1"));
    passport.personal_data.insert(
        "repairHistoryUrl".into(),
        PersonalDataStatement::NothingHeld,
    );
    validate_passport(&passport).expect("stated, and otherwise valid");
}
