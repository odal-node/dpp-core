//! Behaviour of the lens registry: chain composition, refusals, and provenance.

use semver::Version;
use serde_json::Value;

use super::*;

pub(super) fn v(s: &str) -> Version {
    s.parse().unwrap()
}

/// A minimal but valid v1 battery record (schema-required fields), plus a
/// rated capacity so the lens has something to derive.
pub(super) fn battery_v1() -> Value {
    serde_json::json!({
        "gtin": "09506000134352",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 45.2,
        "ratedCapacityKwh": 4.8
    })
}

#[test]
fn identity_view_for_same_version_is_lossless() {
    let lenses = LensRegistry::new();
    let data = battery_v1();
    let derived = lenses
        .upcast("battery", &data, &v("1.0.0"), &v("1.0.0"))
        .unwrap();
    assert!(derived.derived);
    assert!(!derived.lossy);
    assert!(derived.lens_chain.is_empty());
    assert_eq!(derived.data, data);
}

#[test]
fn downcast_is_refused() {
    let lenses = LensRegistry::new();
    let err = lenses
        .upcast("battery", &battery_v1(), &v("2.0.0"), &v("1.0.0"))
        .unwrap_err();
    assert!(matches!(err, UpcastError::NotAnUpcast { .. }));
}

#[test]
fn missing_hop_is_a_typed_refusal_not_silent_identity() {
    let lenses = LensRegistry::new();
    // No battery v2 → v3 lens is registered.
    let err = lenses
        .upcast("battery", &battery_v1(), &v("1.0.0"), &v("3.0.0"))
        .unwrap_err();
    assert!(matches!(err, UpcastError::NoPath { .. }));
}

// ── Composition + loss propagation, exercised with synthetic lenses ──────

fn add_a(v: &Value) -> Result<Value, LensError> {
    let mut out = v.clone();
    out.as_object_mut()
        .ok_or_else(|| LensError("not an object".into()))?
        .insert("a".into(), Value::Bool(true));
    Ok(out)
}

fn add_b_lossy(v: &Value) -> Result<Value, LensError> {
    let mut out = v.clone();
    let obj = out
        .as_object_mut()
        .ok_or_else(|| LensError("not an object".into()))?;
    obj.insert("b".into(), Value::Bool(true));
    obj.remove("dropped"); // this hop is lossy: it discards a field
    Ok(out)
}

#[test]
fn multi_hop_chain_composes_and_propagates_loss() {
    let reg = LensRegistry::from_lenses(vec![
        Lens::new("demo", v("1.0.0"), v("2.0.0"), false, "add a", add_a),
        Lens::new(
            "demo",
            v("2.0.0"),
            v("3.0.0"),
            true,
            "add b, drop",
            add_b_lossy,
        ),
    ]);
    let data = serde_json::json!({ "dropped": 1 });
    let derived = reg.upcast("demo", &data, &v("1.0.0"), &v("3.0.0")).unwrap();

    assert_eq!(derived.data["a"], Value::Bool(true));
    assert_eq!(derived.data["b"], Value::Bool(true));
    assert!(derived.data.get("dropped").is_none());
    assert!(derived.lossy, "a lossy hop must mark the whole chain lossy");
    assert_eq!(
        derived.lens_chain,
        vec![
            ["1.0.0".to_string(), "2.0.0".to_string()],
            ["2.0.0".to_string(), "3.0.0".to_string()],
        ]
    );
}

// ── upcast_toward ───────────────────────────────────────────────────────

#[test]
fn toward_reaches_the_newest_registered_version_short_of_the_target() {
    // Battery's registered lens ends at 2.0.0 while the schema has since
    // moved on additively, so no chain lands on the current version and
    // none should have to: `upcast` refuses the gap outright, and a reader
    // that only needs the data readable must not inherit that refusal.
    let reg = LensRegistry::new();
    let current: Version = crate::catalog::ProductGroupCatalog::new()
        .current_schema_version("battery")
        .expect("battery is in the catalog")
        .parse()
        .expect("catalog versions are semver");
    assert!(
        current > v("2.0.0"),
        "this test is only meaningful while battery's current version is \
         past its last lens — got {current}"
    );

    assert!(matches!(
        reg.upcast("battery", &battery_v1(), &v("1.0.0"), &current),
        Err(UpcastError::NoPath { .. })
    ));

    let derived = reg
        .upcast_toward("battery", &battery_v1(), &v("1.0.0"), &current)
        .expect("the 1.0.0 -> 2.0.0 hop must still be applied");

    // It reports the version actually reached, not the one asked for, and
    // the hop really ran.
    assert_eq!(derived.to, "2.0.0");
    assert_eq!(derived.from, "1.0.0");
    assert_eq!(
        derived.lens_chain,
        vec![["1.0.0".to_string(), "2.0.0".to_string()]]
    );
    assert_eq!(derived.data["ratedEnergyWh"].as_f64(), Some(4800.0));
}

#[test]
fn toward_never_overshoots_the_target() {
    let reg = LensRegistry::from_lenses(vec![
        Lens::new("demo", v("1.0.0"), v("2.0.0"), false, "add a", add_a),
        Lens::new("demo", v("2.0.0"), v("3.0.0"), false, "add b", add_b_lossy),
    ]);
    let data = serde_json::json!({ "dropped": 1 });

    // 2.0.0 is reachable and is the ceiling; the 3.0.0 hop must not run.
    let derived = reg
        .upcast_toward("demo", &data, &v("1.0.0"), &v("2.5.0"))
        .expect("2.0.0 is reachable and below the ceiling");
    assert_eq!(derived.to, "2.0.0");
    assert_eq!(derived.data["a"], Value::Bool(true));
    assert!(derived.data.get("b").is_none());
}

#[test]
fn toward_still_refuses_a_gap_no_lens_touches_at_all() {
    // Making progress optional would turn every unbridgeable gap into a
    // silent identity, which is the one thing this module refuses to do.
    //
    // The gap is synthetic on purpose. This asserts a property of the
    // registry, not a fact about any product group's lens coverage — pointing it at
    // a real gap makes it fail the day someone legitimately bridges that gap,
    // which is what happened when textile 1.0.0 → 1.1.0 was added.
    let reg = LensRegistry::from_lenses(vec![Lens::new(
        "demo",
        v("2.0.0"),
        v("3.0.0"),
        false,
        "add b",
        add_b_lossy,
    )]);
    let err = reg
        .upcast_toward("demo", &serde_json::json!({}), &v("1.0.0"), &v("3.0.0"))
        .unwrap_err();
    assert!(
        matches!(err, UpcastError::NoPath { .. }),
        "nothing leaves demo 1.0.0, so this must stay a typed refusal"
    );
}

#[test]
fn toward_refuses_a_downcast() {
    let reg = LensRegistry::new();
    assert!(matches!(
        reg.upcast_toward("battery", &battery_v1(), &v("2.0.0"), &v("1.0.0")),
        Err(UpcastError::NotAnUpcast { .. })
    ));
}

#[test]
fn toward_is_the_identity_for_the_same_version() {
    // No gap means no progress to require — refusing here would make a
    // product group with no lenses at all unreadable at its own current version.
    let reg = LensRegistry::new();
    let data = battery_v1();
    let derived = reg
        .upcast_toward("battery", &data, &v("2.0.0"), &v("2.0.0"))
        .expect("same version is the identity, not a missing path");
    assert!(derived.lens_chain.is_empty());
    assert!(!derived.lossy);
    assert_eq!(derived.to, "2.0.0");
    assert_eq!(derived.data, data);
}

#[test]
fn upcast_str_tolerates_v_prefix_and_refuses_garbage() {
    let reg = LensRegistry::new();
    let data = battery_v1();
    assert!(reg.upcast_str("battery", &data, "v1.0.0", "v2.0.0").is_ok());
    // An unparseable version is a typed refusal, never a silent identity.
    assert!(matches!(
        reg.upcast_str("battery", &data, "1.0.0", "two"),
        Err(UpcastError::BadVersion(_))
    ));
}
