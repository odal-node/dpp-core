//! Sector-data validation: JSON Schema (via the versioned registry) plus
//! cross-field regulatory rules that JSON Schema cannot express.

use std::sync::OnceLock;

use semver::Version;

use super::validator::SectorValidatorRegistry;
use crate::catalog::SectorCatalog;
use crate::domain::field_error::{FieldError, ValidationErrors};
use crate::domain::sector::{
    SectorData, SvhcSubstance, battery_recycled_chemistry_conflicts,
    validate_battery_operating_temp, validate_fibre_composition, validate_surfactants,
    validate_svhc_substances,
};
use crate::schemas::VersionedSchemaRegistry;

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

/// Validate `sector_data` against the appropriate JSON Schema and any
/// sector-specific cross-field rules (e.g. fibre composition sum).
///
/// The JSON-Schema step resolves against the crate's **embedded** schema
/// registry and catalog (built once at first use). Schemas registered at
/// runtime into a separate [`VersionedSchemaRegistry`] are not visible here —
/// validate those through that registry directly (its fail-closed
/// `validate_strict`).
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

    if let Some(version_str) = catalog.current_schema_version(sector_key) {
        match version_str.parse::<semver::Version>() {
            Ok(version) => {
                if let Err(ve) = default_registry().validate(sector_key, &version, data) {
                    errors.extend(ve.errors);
                }
            }
            // Fail closed: a registered sector with an unparseable current
            // version must not silently skip schema validation.
            Err(_) => errors.push(FieldError {
                field: "/schemaVersion".to_owned(),
                message: format!(
                    "sector '{sector_key}' has an invalid current schema version '{version_str}'"
                ),
            }),
        }
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
    // A catalog entry whose current version won't parse is a misconfiguration,
    // not a reason to skip validation — surface it rather than fail open.
    let version = match version_str.parse::<Version>() {
        Ok(v) => v,
        Err(_) => {
            errors.push(FieldError {
                field: "/schemaVersion".to_owned(),
                message: format!(
                    "sector '{key}' has an invalid current schema version '{version_str}'"
                ),
            });
            return;
        }
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
        SectorData::Battery(d) => {
            // Operating temperature range must be physically coherent (min < max).
            if let Err(msg) =
                validate_battery_operating_temp(d.operating_temp_min_c, d.operating_temp_max_c)
            {
                errors.push(FieldError {
                    field: "/operatingTempMinC".to_owned(),
                    message: msg,
                });
            }
            // Recycled content declared for a metal the chemistry does not contain
            // is a data-integrity contradiction (e.g. cobalt on LFP).
            let chemistry = serde_json::to_value(&d.battery_chemistry)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            for metal in battery_recycled_chemistry_conflicts(
                &chemistry,
                d.recycled_content_cobalt_pct,
                d.recycled_content_lithium_pct,
                d.recycled_content_nickel_pct,
                d.recycled_content_lead_pct,
            ) {
                let field = match metal {
                    "cobalt" => "/recycledContentCobaltPct",
                    "lithium" => "/recycledContentLithiumPct",
                    "nickel" => "/recycledContentNickelPct",
                    "lead" => "/recycledContentLeadPct",
                    _ => "/recycledContent",
                };
                errors.push(FieldError {
                    field: field.to_owned(),
                    message: format!(
                        "{metal} recycled content declared for a {chemistry} battery, \
                         which contains no {metal}"
                    ),
                });
            }
        }
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
