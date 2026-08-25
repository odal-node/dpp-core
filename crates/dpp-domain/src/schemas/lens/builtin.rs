//! The compiled-in lens catalogue and the transforms it is built from.

use semver::Version;
use serde_json::Value;

use super::transform::{Lens, LensError};

/// The compiled-in lenses shipped with core, versioned alongside the schemas
/// they bridge.
pub(super) fn builtin_lenses() -> Vec<Lens> {
    vec![
        Lens::new(
            "battery",
            Version::new(1, 0, 0),
            Version::new(2, 0, 0),
            false,
            "EU Battery Regulation 2023/1542 Annex XIII v2.0.0: derives ratedEnergyWh (Wh) \
             from v1 ratedCapacityKwh (kWh); every other v2 field is an optional addition.",
            battery_v1_to_v2,
        ),
        Lens::new(
            "battery",
            Version::new(2, 4, 0),
            Version::new(2, 5, 0),
            false,
            "EU Battery Regulation 2023/1542 Annex VI Part A point 2 (via Annex XIII \
             point 1(a)) v2.5.0: batteryType becomes required and closed. A v2.4.0 \
             record with no batteryType predates the mandate and cannot be upgraded \
             without inventing a value, so this hop refuses rather than default one.",
            battery_v2_4_to_v2_5,
        ),
        Lens::new(
            "battery",
            Version::new(2, 5, 0),
            Version::new(2, 6, 0),
            false,
            "EU Battery Regulation 2023/1542 Annex XIII point 4 v2.6.0: adds the \
             individual-battery tier — dynamicPerformance (4(a)), batteryStatus (4(c)) \
             and usageHistory (4(d)) — all optional, and relaxes \
             expectedLifetimeCycles out of required, since point 1(j) reaches \
             industrial batteries only where lifetime can be expressed in cycles. \
             Also splits Annex XIII point 1(n) into its two figures and point 1(o) \
             into cell and pack resistance; the 1(o) hop refuses rather than guess \
             which measurement a single stored value was.",
            battery_v2_5_to_v2_6,
        ),
        Lens::new(
            "electronics",
            Version::new(1, 1, 0),
            Version::new(1, 2, 0),
            false,
            "Regulation (EU) 2023/1670 Art. 1(1) v1.2.0: productCategory is narrowed to the \
             four device types the regulation actually enumerates (smartphone, other mobile \
             phone, cordless phone, slate tablet). A v1.1.0 record declaring one of the seven \
             removed values has no lawful category to upgrade into, so this hop refuses.",
            electronics_v1_1_to_v1_2,
        ),
        Lens::new(
            "steel",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-product_group naming consistency: renames countryOfProduction to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_production,
        ),
        Lens::new(
            "aluminium",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-product_group naming consistency: renames countryOfProduction to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_production,
        ),
        Lens::new(
            "construction",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-product_group naming consistency: renames countryOfManufacture to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacture,
        ),
        Lens::new(
            "detergent",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-product_group naming consistency: renames countryOfManufacture to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacture,
        ),
        Lens::new(
            "furniture",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-product_group naming consistency: renames countryOfManufacture to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacture,
        ),
        Lens::new(
            "toy",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-product_group naming consistency: renames countryOfManufacture to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacture,
        ),
        Lens::new(
            "textile",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "v1.1.0 adds sixteen optional fields (SCIP/SVHC disclosure, per-fibre \
             origin, durability, microplastic shedding) and changes nothing that \
             already existed: the two versions declare identical `required` lists \
             and v1.1.0 removes no property. Purely additive, so the document \
             passes through untouched — but the lens must exist, because a chain \
             cannot cross a gap, and without it a v1.0.0 document has no path to \
             the current version and cannot be read at all.",
            identity,
        ),
        Lens::new(
            "textile",
            Version::new(1, 1, 0),
            Version::new(1, 2, 0),
            false,
            "Cross-product_group naming consistency: renames countryOfManufacturing to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacturing,
        ),
    ]
}

/// A schema step that added only optional fields: the document is already valid
/// under the later version, so it passes through untouched.
///
/// Not redundant. `upcast_str_toward` walks a chain of lenses, and a missing
/// step breaks the whole chain — a document two versions behind cannot reach the
/// current version through a gap, even when the gap itself requires no change.
fn identity(v: &Value) -> Result<Value, LensError> {
    Ok(v.clone())
}

/// Renames a top-level JSON key to `countryOfOrigin`, leaving every other
/// field untouched. Shared by the country-of-origin naming-unification lenses
/// above — each product group's old key differs, so the key name is a parameter.
fn rename_country_field(v: &Value, old_key: &str) -> Result<Value, LensError> {
    let mut out = v.clone();
    let obj = out
        .as_object_mut()
        .ok_or_else(|| LensError("product_group data must be a JSON object".to_owned()))?;
    if let Some(val) = obj.remove(old_key) {
        obj.insert("countryOfOrigin".to_owned(), val);
    }
    Ok(out)
}

fn rename_country_of_production(v: &Value) -> Result<Value, LensError> {
    rename_country_field(v, "countryOfProduction")
}

fn rename_country_of_manufacture(v: &Value) -> Result<Value, LensError> {
    rename_country_field(v, "countryOfManufacture")
}

fn rename_country_of_manufacturing(v: &Value) -> Result<Value, LensError> {
    rename_country_field(v, "countryOfManufacturing")
}

/// Battery `v1.0.0 → v2.0.0`: pass all fields through, and derive `ratedEnergyWh`
/// (watt-hours) from `ratedCapacityKwh` (kilowatt-hours) when present. Lossless —
/// v2 is a strict superset whose only computable field from v1 data is the
/// watt-hour restatement of the kilowatt-hour rating.
fn battery_v1_to_v2(v1: &Value) -> Result<Value, LensError> {
    let mut out = v1.clone();
    let obj = out
        .as_object_mut()
        .ok_or_else(|| LensError("battery product_group data must be a JSON object".to_owned()))?;
    if let Some(kwh) = obj.get("ratedCapacityKwh").and_then(Value::as_f64)
        && !obj.contains_key("ratedEnergyWh")
    {
        // Restate kWh as Wh, stripping f64 noise (e.g. 100.00000000000001 → 100.0)
        // while keeping up to 6 real decimals.
        let wh = (kwh * 1000.0 * 1_000_000.0).round() / 1_000_000.0;
        obj.insert("ratedEnergyWh".to_owned(), serde_json::json!(wh));
    }
    Ok(out)
}

/// Battery `v2.4.0 → v2.5.0`: passes every field through unchanged. Refuses,
/// rather than defaulting a value, when `batteryType` is absent or not a
/// string — the field becomes required at v2.5.0, and a record written before
/// the mandate existed cannot be made to satisfy it without inventing a
/// category the manufacturer never declared.
fn battery_v2_4_to_v2_5(v: &Value) -> Result<Value, LensError> {
    let obj = v
        .as_object()
        .ok_or_else(|| LensError("battery product_group data must be a JSON object".to_owned()))?;
    match obj.get("batteryType") {
        Some(Value::String(_)) => Ok(v.clone()),
        _ => Err(LensError(
            "batteryType is required from v2.5.0 (EU 2023/1542 Annex VI Part A point 2 \
             via Annex XIII point 1(a)); this record predates the mandate and has none, \
             so it cannot be upgraded"
                .to_owned(),
        )),
    }
}

/// Battery `v2.5.0 → v2.6.0`: additions pass through, one key is renamed, and
/// one **refuses**.
///
/// The additive half is straightforward. `dynamicPerformance`, `batteryStatus`
/// and `usageHistory` are optional, and `expectedLifetimeCycles` only *stops*
/// being required, which no existing record can fail. Absence stays absent —
/// materialising an empty point-4 block would assert that the battery reported
/// measurements it never reported.
///
/// **`roundTripEfficiencyPct` and `internalResistanceMohm` are carried
/// verbatim.** Annex XIII splits each into a pair at v2.6.0 — 1(n) into the
/// initial figure and the one at 50 % of cycle-life, 1(o) into cell and pack —
/// but neither legacy value can be assigned to a half of its pair. The first
/// was documented against "50% state of charge", a condition 1(n) does not
/// state; the second cannot say whether it was the cell or the pack. Both keys
/// therefore survive into v2.6.0 under their own names, marked legacy, and the
/// successor fields carry new declarations only. Moving a value would invent
/// the very distinction the split exists to record.
fn battery_v2_5_to_v2_6(v: &Value) -> Result<Value, LensError> {
    if !v.is_object() {
        return Err(LensError(
            "battery product_group data must be a JSON object".to_owned(),
        ));
    }
    Ok(v.clone())
}

/// Electronics `v1.1.0 → v1.2.0`: passes every field through unchanged.
/// Refuses, rather than dropping or substituting the record's own category,
/// when `productCategory` is not one of the four device types Regulation
/// (EU) 2023/1670 Art. 1(1) actually enumerates — a record declaring
/// `laptop`, `tv`, or any of the other removed values was written against a
/// category this product group never had a lawful basis for, and there is no value
/// to substitute that would not misdescribe the product.
fn electronics_v1_1_to_v1_2(v: &Value) -> Result<Value, LensError> {
    const VALID: [&str; 4] = [
        "smartphone",
        "other-mobile-phone",
        "cordless-phone",
        "tablet",
    ];
    let obj = v.as_object().ok_or_else(|| {
        LensError("electronics product_group data must be a JSON object".to_owned())
    })?;
    match obj.get("productCategory") {
        Some(Value::String(s)) if VALID.contains(&s.as_str()) => Ok(v.clone()),
        _ => Err(LensError(
            "productCategory is not one of the four device types Regulation (EU) 2023/1670 \
             Art. 1(1) enumerates (smartphone, other-mobile-phone, cordless-phone, tablet); \
             this record predates the narrowing and cannot be upgraded"
                .to_owned(),
        )),
    }
}
