//! Battery Annex VII: the Part A state-of-health lists and the Part B expected
//! lifetime items, and which audience each reaches.

// ── Annex VII Part A state of health ─────────────────────────────────────────

#[test]
fn state_of_health_round_trips_both_annex_vii_parameter_sets() {
    use crate::product_group::StateOfHealth;

    let ev = StateOfHealth::ElectricVehicle { soce_pct: 92.5 };
    let json = serde_json::to_value(&ev).unwrap();
    assert_eq!(json["parameterSet"], "electricVehicle");
    assert_eq!(json["socePct"], 92.5);
    // SOCE is the only Annex VII Part A parameter for EV batteries.
    assert_eq!(json.as_object().unwrap().len(), 2);
    assert_eq!(serde_json::from_value::<StateOfHealth>(json).unwrap(), ev);

    let stationary = StateOfHealth::StationaryOrLmt {
        remaining_capacity_pct: 88.0,
        remaining_power_capability_pct: None,
        remaining_round_trip_efficiency_pct: Some(94.0),
        self_discharge_rate_pct_per_month: 1.5,
        ohmic_resistance_mohm: None,
    };
    let json = serde_json::to_value(&stationary).unwrap();
    assert_eq!(json["parameterSet"], "stationaryOrLmt");
    // The two unconditional items are always present; the three "where
    // possible" items are omitted when absent rather than serialised as null.
    assert_eq!(json["remainingCapacityPct"], 88.0);
    assert_eq!(json["selfDischargeRatePctPerMonth"], 1.5);
    assert!(json.get("remainingPowerCapabilityPct").is_none());
    assert!(json.get("ohmicResistanceMohm").is_none());
    assert_eq!(
        serde_json::from_value::<StateOfHealth>(json).unwrap(),
        stationary
    );
}

#[test]
fn annex_vii_unconditional_parameters_are_required() {
    use crate::product_group::StateOfHealth;

    // Annex VII Part A qualifies items 2, 3 and 5 with "where possible" but not
    // items 1 and 4, so the latter two have no serde default and a payload
    // omitting either must fail to deserialize.
    let missing_self_discharge = serde_json::json!({
        "parameterSet": "stationaryOrLmt",
        "remainingCapacityPct": 88.0,
    });
    assert!(serde_json::from_value::<StateOfHealth>(missing_self_discharge).is_err());

    let missing_soce = serde_json::json!({ "parameterSet": "electricVehicle" });
    assert!(serde_json::from_value::<StateOfHealth>(missing_soce).is_err());

    // Omitting only the "where possible" items is valid.
    let minimal = serde_json::json!({
        "parameterSet": "stationaryOrLmt",
        "remainingCapacityPct": 88.0,
        "selfDischargeRatePctPerMonth": 1.5,
    });
    assert!(serde_json::from_value::<StateOfHealth>(minimal).is_ok());
}

#[test]
fn state_of_health_is_withheld_from_authorities() {
    // Annex XIII point 4(b) puts state of health in the individual-battery set,
    // which Art. 77(2)(b) does not grant to notified bodies or market
    // surveillance. Before this change the field was unclassified and therefore
    // public — this asserts the classification, through the catalog.
    use crate::catalog::ProductGroupCatalog;
    use crate::identity::{Audience, Disclosure};

    let catalog = ProductGroupCatalog::new();
    let battery = catalog.get("battery").expect("battery in catalog");
    for field in ["stateOfHealth", "stateOfHealthPct"] {
        assert_eq!(
            battery.disclosure.get(field),
            Some(&Disclosure::Individual),
            "{field} must be individual-battery data"
        );
    }
    assert!(!Audience::Public.may_see(Disclosure::Individual));
    assert!(!Audience::Authority.may_see(Disclosure::Individual));
    assert!(Audience::LegitimateInterest.may_see(Disclosure::Individual));
}

// ── Annex VII Part B expected lifetime ───────────────────────────────────────

#[test]
fn expected_lifetime_round_trips_with_illustrative_harmful_events() {
    use crate::product_group::{ExpectedLifetime, HarmfulEvents};

    let lifetime = ExpectedLifetime {
        put_into_service_date: chrono::NaiveDate::from_ymd_opt(2027, 3, 1),
        energy_throughput_kwh: 12_500.0,
        capacity_throughput_ah: 48_000.0,
        harmful_events: HarmfulEvents {
            deep_discharge_events: Some(4),
            hours_in_extreme_temperature: Some(31.5),
            // Annex VII Part B item 4 lists its examples "such as", so an
            // untracked one is an omission, not a malformed record.
            hours_charging_in_extreme_temperature: None,
        },
        full_equivalent_cycles: 812.0,
    };

    let json = serde_json::to_value(&lifetime).unwrap();
    assert_eq!(json["energyThroughputKwh"], 12_500.0);
    assert_eq!(json["capacityThroughputAh"], 48_000.0);
    assert_eq!(json["fullEquivalentCycles"], 812.0);
    assert_eq!(json["harmfulEvents"]["deepDischargeEvents"], 4);
    assert!(
        json["harmfulEvents"]
            .get("hoursChargingInExtremeTemperature")
            .is_none(),
        "an untracked harmful event is omitted, not null"
    );
    assert_eq!(
        serde_json::from_value::<ExpectedLifetime>(json).unwrap(),
        lifetime
    );
}

#[test]
fn part_b_unconditional_items_are_required() {
    use crate::product_group::ExpectedLifetime;

    // Only item 1's "date of putting into service" is qualified ("where
    // appropriate"). Items 2, 3 and 5 are unconditional.
    let missing_throughput = serde_json::json!({
        "capacityThroughputAh": 1.0,
        "harmfulEvents": {},
        "fullEquivalentCycles": 1.0,
    });
    assert!(serde_json::from_value::<ExpectedLifetime>(missing_throughput).is_err());

    let minimal = serde_json::json!({
        "energyThroughputKwh": 1.0,
        "capacityThroughputAh": 1.0,
        "harmfulEvents": {},
        "fullEquivalentCycles": 1.0,
    });
    assert!(serde_json::from_value::<ExpectedLifetime>(minimal).is_ok());
}

#[test]
fn measured_lifetime_is_individual_but_the_design_figure_stays_public() {
    // Annex XIII point 1(j) makes the model-level "expected battery lifetime
    // expressed in cycles" public; point 4(d) restricts the measured per-item
    // use data to persons with a legitimate interest. Two different fields,
    // two different classes — collapsing them would either hide a public figure
    // or publish an individual battery's usage history.
    use crate::catalog::ProductGroupCatalog;
    use crate::identity::{Audience, Disclosure};

    let catalog = ProductGroupCatalog::new();
    let battery = catalog.get("battery").expect("battery in catalog");

    assert_eq!(
        battery.disclosure.get("expectedLifetime"),
        Some(&Disclosure::Individual),
        "measured Part B data is Annex XIII point 4(d)"
    );
    assert_eq!(
        battery.disclosure.get("expectedLifetimeCycles"),
        None,
        "the model-level design figure is public under Annex XIII point 1(j)"
    );
    assert!(!Audience::Authority.may_see(Disclosure::Individual));
    assert!(Audience::LegitimateInterest.may_see(Disclosure::Individual));
}
