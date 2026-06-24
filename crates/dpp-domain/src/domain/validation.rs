//! JSON Schema + cross-field validation for sector-specific DPP data.
//!
//! The schema step routes through the shared [`VersionedSchemaRegistry`] at the
//! version the [`SectorCatalog`] marks current for the sector — there are no
//! per-sector validators and no hardcoded versions here. Cross-field regulatory
//! rules (which JSON Schema cannot express, e.g. "fibre percentages sum to
//! ~100%") come from `dpp-rules` via the `dpp-domain` adapters.
//! See `docs/architecture/SECTOR-MODEL-CONSOLIDATION.md` (step C2).
//!
//! **Note**: excluded from wasm32 builds since jsonschema depends on reqwest's
//! blocking API.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::OnceLock;

use semver::Version;

use crate::catalog::SectorCatalog;
use crate::domain::field_error::{FieldError, ValidationErrors};
use crate::domain::sector::{
    SectorData, SvhcSubstance, validate_fibre_composition, validate_surfactants,
    validate_svhc_substances,
};
use crate::schemas::VersionedSchemaRegistry;

// ─── Extensibility: runtime sector validators ─────────────────────────────────

/// Trait for runtime-registered sector validators.
///
/// Register an implementation in [`SectorValidatorRegistry`] to provide JSON
/// Schema + cross-field validation for sectors that are not known to this crate
/// at compile time (e.g., plugin-defined sectors carrying `SectorData::Other`).
pub trait SectorValidator: Send + Sync {
    /// Validate the sector payload (the inner data, without the `"sector"` tag key).
    fn validate(&self, data: &serde_json::Value) -> Result<(), Vec<FieldError>>;
}

/// Registry of runtime sector validators, keyed by catalog sector key.
///
/// An empty registry (the default) causes `SectorData::Other` to fail
/// validation with an "unknown sector" error — silent pass-through is not safe.
#[derive(Default)]
pub struct SectorValidatorRegistry {
    validators: std::collections::HashMap<String, std::sync::Arc<dyn SectorValidator>>,
}

impl SectorValidatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        key: impl Into<String>,
        validator: std::sync::Arc<dyn SectorValidator>,
    ) {
        self.validators.insert(key.into(), validator);
    }

    fn get(&self, key: &str) -> Option<&dyn SectorValidator> {
        self.validators.get(key).map(std::sync::Arc::as_ref)
    }
}

// ─── Process-wide defaults ─────────────────────────────────────────────────────

/// The embedded schema registry, built once.
fn default_registry() -> &'static VersionedSchemaRegistry {
    static REGISTRY: OnceLock<VersionedSchemaRegistry> = OnceLock::new();
    REGISTRY.get_or_init(VersionedSchemaRegistry::new)
}

/// The embedded sector catalog, built once.
fn default_catalog() -> &'static SectorCatalog {
    static CATALOG: OnceLock<SectorCatalog> = OnceLock::new();
    CATALOG.get_or_init(SectorCatalog::new)
}

// `FieldError` and `ValidationErrors` moved to `crate::domain::field_error`
// (wasm-safe) so `DppError` can carry structured validation detail. Imported above.

// ─── Public API ─────────────────────────────────────────────────────────────────

/// Validate `sector_data` against the appropriate JSON Schema and any
/// sector-specific cross-field rules (e.g. fibre composition sum).
///
/// `SectorData::Other` is a **hard error** here — pass a
/// [`SectorValidatorRegistry`] via [`validate_sector_data_with_registry`] to
/// handle runtime-registered sectors.
///
/// # Errors
///
/// Returns `ValidationErrors` listing every failing field when validation
/// fails. The `Ok(())` path means the data is structurally valid.
pub fn validate_sector_data(sector_data: &SectorData) -> Result<(), ValidationErrors> {
    validate_sector_data_with_registry(sector_data, &SectorValidatorRegistry::default())
}

/// Like [`validate_sector_data`] but accepts a runtime validator registry.
///
/// For `SectorData::Other(v)`, dispatches to the validator registered under
/// key `"other"` in `registry`. Returns a hard error if no validator is
/// registered for that key.
pub fn validate_sector_data_with_registry(
    sector_data: &SectorData,
    registry: &SectorValidatorRegistry,
) -> Result<(), ValidationErrors> {
    let mut errors: Vec<FieldError> = Vec::new();
    if let SectorData::Other(_) = sector_data {
        match registry.get("other") {
            Some(v) => {
                if let Err(field_errors) = v.validate(&sector_data_instance(sector_data)) {
                    errors.extend(field_errors);
                }
            }
            None => errors.push(FieldError {
                field: "/sector".to_owned(),
                message: "SectorData::Other cannot be validated without a registered validator; \
                          pass a SectorValidatorRegistry with an \"other\" entry"
                    .to_owned(),
            }),
        }
    } else {
        schema_errors(sector_data, &mut errors);
        cross_field_errors(sector_data, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrors { errors })
    }
}

/// Validate raw sector JSON using the embedded schema registry and any
/// runtime cross-field validator.
///
/// This is the extension point for the plugin host: when a plugin produces a
/// DPP with a sector key not present in the compile-time `SectorData` enum,
/// pass the raw JSON through this function with an appropriate
/// `SectorValidatorRegistry`.
///
/// Validation steps:
/// 1. JSON Schema — resolved via the embedded [`SectorCatalog`] and
///    [`VersionedSchemaRegistry`] (for sectors with a registered schema).
/// 2. Cross-field — dispatched to `registry.get(sector_key)` when present.
/// 3. Hard error — if neither a schema nor a registered validator exists for
///    `sector_key`.
pub fn validate_raw_sector_data(
    sector_key: &str,
    data: &serde_json::Value,
    registry: &SectorValidatorRegistry,
) -> Result<(), ValidationErrors> {
    let mut errors: Vec<FieldError> = Vec::new();
    let catalog = default_catalog();
    let has_schema = catalog.current_schema_version(sector_key).is_some();

    if let Some(version_str) = catalog.current_schema_version(sector_key)
        && let Ok(version) = version_str.parse::<semver::Version>()
        && let Err(ve) = default_registry().validate(sector_key, &version, data)
    {
        errors.extend(ve.errors);
    }

    match registry.get(sector_key) {
        Some(v) => {
            if let Err(field_errors) = v.validate(data) {
                errors.extend(field_errors);
            }
        }
        None if !has_schema => {
            errors.push(FieldError {
                field: "/sector".to_owned(),
                message: format!(
                    "unknown sector \"{sector_key}\": no JSON schema or cross-field validator registered"
                ),
            });
        }
        None => {}
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrors { errors })
    }
}

/// Schema validation via the registry at the catalog-resolved current version.
fn schema_errors(sector_data: &SectorData, errors: &mut Vec<FieldError>) {
    let key = sector_data.sector().catalog_key();
    // No catalog entry (e.g. `Other`) → no schema to validate against.
    let Some(version_str) = default_catalog().current_schema_version(key) else {
        return;
    };
    let Ok(version) = version_str.parse::<Version>() else {
        return;
    };
    let instance = sector_data_instance(sector_data);
    if let Err(ve) = default_registry().validate(key, &version, &instance) {
        errors.extend(ve.errors);
    }
}

/// The JSON the schema expects: the inner sector fields without the `"sector"`
/// discriminant tag that `SectorData` serialises (schemas forbid extra props).
fn sector_data_instance(sector_data: &SectorData) -> serde_json::Value {
    let mut value = serde_json::to_value(sector_data).expect("SectorData serializes to Value");
    if let Some(obj) = value.as_object_mut() {
        obj.remove("sector");
    }
    value
}

/// Cross-field regulatory rules that JSON Schema cannot express, delegated to
/// `dpp-rules` through the `dpp-domain` adapters.
fn cross_field_errors(sector_data: &SectorData, errors: &mut Vec<FieldError>) {
    match sector_data {
        SectorData::Textile(d) => {
            if let Err(msg) = validate_fibre_composition(&d.fibre_composition) {
                errors.push(FieldError {
                    field: "/fibreComposition".to_owned(),
                    message: msg,
                });
            }
            push_svhc(d.svhc_substances.as_deref(), errors);
            if let Some(ds) = d.durability_score
                && !(0.0..=10.0).contains(&ds)
            {
                errors.push(FieldError {
                    field: "/durabilityScore".to_owned(),
                    message: format!("durability_score {ds} must be 0.0–10.0"),
                });
            }
        }
        SectorData::Electronics(d) => push_svhc(d.svhc_substances.as_deref(), errors),
        SectorData::Toy(d) => push_svhc(d.svhc_substances.as_deref(), errors),
        SectorData::Furniture(d) => push_svhc(d.svhc_substances.as_deref(), errors),
        SectorData::Detergent(d) => {
            if let Err(msg) = validate_surfactants(&d.surfactants) {
                errors.push(FieldError {
                    field: "/surfactants".to_owned(),
                    message: msg,
                });
            }
        }
        _ => {}
    }
}

fn push_svhc(substances: Option<&[SvhcSubstance]>, errors: &mut Vec<FieldError>) {
    if let Some(s) = substances
        && let Err(msg) = validate_svhc_substances(s)
    {
        errors.push(FieldError {
            field: "/svhcSubstances".to_owned(),
            message: msg,
        });
    }
}

// ─── Batch validation ────────────────────────────────────────────────────────

/// Result of validating a single item in a batch.
#[derive(Debug, Clone)]
pub struct BatchValidationItem {
    /// Zero-based index in the input slice.
    pub index: usize,
    /// Validation result: `Ok(())` if valid, `Err` with field-level errors otherwise.
    pub result: Result<(), ValidationErrors>,
}

/// Validate a batch of sector data items, collecting all errors per item.
///
/// The returned `Vec` has the same length and order as the input.
pub fn validate_sector_data_batch(items: &[SectorData]) -> Vec<BatchValidationItem> {
    items
        .iter()
        .enumerate()
        .map(|(index, data)| BatchValidationItem {
            index,
            result: validate_sector_data(data),
        })
        .collect()
}

/// Returns only the failures from a batch validation run.
pub fn batch_errors(results: &[BatchValidationItem]) -> Vec<&BatchValidationItem> {
    results.iter().filter(|item| item.result.is_err()).collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gtin::Gtin;
    use crate::domain::sector::{BatteryChemistry, BatteryData, FibreEntry, TextileData};

    fn valid_battery() -> SectorData {
        SectorData::Battery(BatteryData {
            gtin: Gtin::parse("09506000134352").unwrap(),
            battery_chemistry: BatteryChemistry::Lfp,
            nominal_voltage_v: 48.0,
            nominal_capacity_ah: 100.0,
            expected_lifetime_cycles: 3000,
            co2e_per_unit_kg: 85.4,
            recycled_content_cobalt_pct: None,
            recycled_content_lithium_pct: None,
            recycled_content_nickel_pct: None,
            state_of_health_pct: None,
            rated_capacity_kwh: None,
            carbon_footprint_class: None,
            due_diligence_url: None,
            cathode_material: None,
            anode_material: None,
            electrolyte_material: None,
            critical_raw_materials: None,
            disassembly_instructions_url: None,
            soh_methodology: None,
            operating_temp_min_c: None,
            operating_temp_max_c: None,
            rated_energy_wh: None,
            recycled_content_lead_pct: None,
            battery_weight_kg: None,
            battery_type: None,
            round_trip_efficiency_pct: None,
            internal_resistance_mohm: None,
            manufacturing_date: None,
            manufacturing_place: None,
            battery_model_id: None,
            battery_passport_number: None,
        })
    }

    fn valid_textile() -> SectorData {
        SectorData::Textile(TextileData {
            fibre_composition: vec![
                FibreEntry {
                    fibre: "cotton".into(),
                    pct: 60.0,
                    country_of_origin: None,
                },
                FibreEntry {
                    fibre: "polyester".into(),
                    pct: 40.0,
                    country_of_origin: None,
                },
            ],
            country_of_manufacturing: "BD".into(),
            care_instructions: "30°C machine wash".into(),
            chemical_compliance_standard: "OEKO-TEX 100".into(),
            recycled_content_pct: None,
            carbon_footprint_kg_co2e: None,
            water_use_litres: None,
            microplastic_shedding_mg_per_wash: None,
            repair_score: None,
            durability_score: None,
            expected_wash_cycles: None,
            country_of_raw_material_origin: None,
            svhc_substances: None,
            allergens: None,
            substances_of_concern: None,
            recyclability_class: None,
            end_of_life_instructions: None,
            reuse_condition: None,
            prior_use_cycles: None,
            disassembly_instructions: None,
            spare_parts_available: None,
            product_weight_grams: None,
            repair_history_url: None,
            repair_count: None,
            pef_score: None,
        })
    }

    #[test]
    fn valid_battery_passes() {
        // Routed through the registry at the catalog's current battery version (v2.0.0).
        assert!(validate_sector_data(&valid_battery()).is_ok());
    }

    #[test]
    fn valid_textile_passes() {
        assert!(validate_sector_data(&valid_textile()).is_ok());
    }

    // The following exercise the schema layer directly through the registry,
    // crafting structurally invalid instances the type system would otherwise
    // prevent.

    #[test]
    fn battery_missing_required_field_fails() {
        let reg = VersionedSchemaRegistry::new();
        let v: Version = "1.0.0".parse().unwrap();
        let instance = serde_json::json!({
            "batteryChemistry": "LFP",
            "nominalVoltageV": 48.0,
            "nominalCapacityAh": 100.0,
            "expectedLifetimeCycles": 3000,
            "co2ePerUnitKg": 85.4
            // "gtin" intentionally missing
        });
        let err = reg.validate("battery", &v, &instance).unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.message.contains("gtin")),
            "expected gtin error, got: {err:?}"
        );
    }

    #[test]
    fn battery_invalid_gtin_pattern_fails() {
        let reg = VersionedSchemaRegistry::new();
        let v: Version = "1.0.0".parse().unwrap();
        let instance = serde_json::json!({
            "gtin": "123", // too short
            "batteryChemistry": "LFP",
            "nominalVoltageV": 48.0,
            "nominalCapacityAh": 100.0,
            "expectedLifetimeCycles": 3000,
            "co2ePerUnitKg": 85.4
        });
        assert!(reg.validate("battery", &v, &instance).is_err());
    }

    #[test]
    fn textile_missing_care_instructions_fails() {
        let reg = VersionedSchemaRegistry::new();
        let v: Version = "1.1.0".parse().unwrap();
        let instance = serde_json::json!({
            "fibreComposition": [{"fibre": "cotton", "pct": 100}],
            "countryOfManufacturing": "BD",
            // "careInstructions" intentionally missing
            "chemicalComplianceStandard": "REACH"
        });
        let err = reg.validate("textile", &v, &instance).unwrap_err();
        assert!(
            err.errors
                .iter()
                .any(|e| e.message.contains("careInstructions")),
            "expected careInstructions error, got: {err:?}"
        );
    }

    #[test]
    fn textile_empty_fibre_composition_fails() {
        let reg = VersionedSchemaRegistry::new();
        let v: Version = "1.1.0".parse().unwrap();
        let instance = serde_json::json!({
            "fibreComposition": [], // minItems: 1
            "countryOfManufacturing": "DE",
            "careInstructions": "dry clean only",
            "chemicalComplianceStandard": "GOTS"
        });
        assert!(reg.validate("textile", &v, &instance).is_err());
    }

    #[test]
    fn textile_fibre_sum_not_100_fails() {
        // Schema passes (pct 0–100 individually); the cross-field rule fails.
        let data = SectorData::Textile(TextileData {
            fibre_composition: vec![
                FibreEntry {
                    fibre: "cotton".into(),
                    pct: 60.0,
                    country_of_origin: None,
                },
                FibreEntry {
                    fibre: "polyester".into(),
                    pct: 30.0, // sums to 90
                    country_of_origin: None,
                },
            ],
            country_of_manufacturing: "PT".into(),
            care_instructions: "Hand wash only".into(),
            chemical_compliance_standard: "REACH".into(),
            recycled_content_pct: None,
            carbon_footprint_kg_co2e: None,
            water_use_litres: None,
            microplastic_shedding_mg_per_wash: None,
            repair_score: None,
            durability_score: None,
            expected_wash_cycles: None,
            country_of_raw_material_origin: None,
            svhc_substances: None,
            allergens: None,
            substances_of_concern: None,
            recyclability_class: None,
            end_of_life_instructions: None,
            reuse_condition: None,
            prior_use_cycles: None,
            disassembly_instructions: None,
            spare_parts_available: None,
            product_weight_grams: None,
            repair_history_url: None,
            repair_count: None,
            pef_score: None,
        });
        let err = validate_sector_data(&data).unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.field == "/fibreComposition"),
            "expected /fibreComposition error, got: {err:?}"
        );
    }

    // ── SectorValidatorRegistry / validate_raw_sector_data tests ─────────────

    #[test]
    fn other_sector_data_fails_without_registry() {
        let data = SectorData::Other(serde_json::json!({"field": "value"}));
        let err = validate_sector_data(&data).unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.field == "/sector"),
            "expected /sector error for Other without registry"
        );
    }

    #[test]
    fn other_sector_data_passes_with_registered_validator() {
        use std::sync::Arc;

        struct AlwaysOkValidator;
        impl SectorValidator for AlwaysOkValidator {
            fn validate(&self, _: &serde_json::Value) -> Result<(), Vec<FieldError>> {
                Ok(())
            }
        }

        let mut registry = SectorValidatorRegistry::new();
        registry.register("other", Arc::new(AlwaysOkValidator));

        let data = SectorData::Other(serde_json::json!({"field": "value"}));
        assert!(
            validate_sector_data_with_registry(&data, &registry).is_ok(),
            "registered AlwaysOkValidator must allow Other sector"
        );
    }

    #[test]
    fn other_sector_data_validator_errors_propagate() {
        use std::sync::Arc;

        struct AlwaysFailValidator;
        impl SectorValidator for AlwaysFailValidator {
            fn validate(&self, _: &serde_json::Value) -> Result<(), Vec<FieldError>> {
                Err(vec![FieldError {
                    field: "/field".to_owned(),
                    message: "injected failure".to_owned(),
                }])
            }
        }

        let mut registry = SectorValidatorRegistry::new();
        registry.register("other", Arc::new(AlwaysFailValidator));

        let data = SectorData::Other(serde_json::json!({"field": "bad"}));
        let err = validate_sector_data_with_registry(&data, &registry).unwrap_err();
        assert!(
            err.errors
                .iter()
                .any(|e| e.message.contains("injected failure")),
            "validator errors must propagate"
        );
    }

    #[test]
    fn validate_raw_sector_data_known_sector_succeeds() {
        // "battery" has an embedded schema — validate known-good raw JSON.
        let data = serde_json::json!({
            "gtin": "09506000134352",
            "batteryChemistry": "LFP",
            "nominalVoltageV": 48.0,
            "nominalCapacityAh": 100.0,
            "expectedLifetimeCycles": 3000,
            "co2ePerUnitKg": 85.4
        });
        let registry = SectorValidatorRegistry::default();
        assert!(validate_raw_sector_data("battery", &data, &registry).is_ok());
    }

    #[test]
    fn validate_raw_sector_data_unknown_sector_fails() {
        let data = serde_json::json!({"field": "value"});
        let registry = SectorValidatorRegistry::default();
        let err = validate_raw_sector_data("nonexistent-sector", &data, &registry).unwrap_err();
        assert!(
            err.errors
                .iter()
                .any(|e| e.message.contains("nonexistent-sector")),
            "expected error naming the unknown sector key"
        );
    }

    #[test]
    fn batch_validation_mixed_results() {
        let items = vec![
            valid_battery(),
            valid_textile(),
            // Invalid: fibre sum != 100
            SectorData::Textile(TextileData {
                fibre_composition: vec![FibreEntry {
                    fibre: "cotton".into(),
                    pct: 50.0,
                    country_of_origin: None,
                }],
                country_of_manufacturing: "PT".into(),
                care_instructions: "Hand wash".into(),
                chemical_compliance_standard: "REACH".into(),
                recycled_content_pct: None,
                carbon_footprint_kg_co2e: None,
                water_use_litres: None,
                microplastic_shedding_mg_per_wash: None,
                repair_score: None,
                durability_score: None,
                expected_wash_cycles: None,
                country_of_raw_material_origin: None,
                svhc_substances: None,
                allergens: None,
                substances_of_concern: None,
                recyclability_class: None,
                end_of_life_instructions: None,
                reuse_condition: None,
                prior_use_cycles: None,
                disassembly_instructions: None,
                spare_parts_available: None,
                product_weight_grams: None,
                repair_history_url: None,
                repair_count: None,
                pef_score: None,
            }),
        ];

        let results = validate_sector_data_batch(&items);
        assert_eq!(results.len(), 3);
        assert!(results[0].result.is_ok());
        assert!(results[1].result.is_ok());
        assert!(results[2].result.is_err());

        let errors = batch_errors(&results);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].index, 2);
    }
}
