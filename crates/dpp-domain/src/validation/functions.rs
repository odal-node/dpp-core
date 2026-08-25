//! ProductGroup-data validation: JSON Schema (via the versioned registry) plus
//! cross-field regulatory rules that JSON Schema cannot express.

use std::sync::OnceLock;

use semver::Version;

use super::rules::{
    battery_recycled_chemistry_conflicts, validate_battery_operating_temp,
    validate_fibre_composition, validate_surfactants, validate_svhc_substances,
};
use super::validator::ProductGroupValidatorRegistry;
use crate::catalog::ProductGroupCatalog;
use crate::error::DppError;
use crate::error::field::{FieldError, ValidationErrors};
use crate::passport::Passport;
use crate::product_group::{ProductGroupData, SvhcSubstance};
use crate::schemas::VersionedSchemaRegistry;

/// The embedded schema registry, built once.
fn default_registry() -> &'static VersionedSchemaRegistry {
    static REGISTRY: OnceLock<VersionedSchemaRegistry> = OnceLock::new();
    REGISTRY.get_or_init(VersionedSchemaRegistry::new)
}

/// The embedded product group catalog, built once.
fn default_catalog() -> &'static ProductGroupCatalog {
    static CATALOG: OnceLock<ProductGroupCatalog> = OnceLock::new();
    CATALOG.get_or_init(ProductGroupCatalog::new)
}

/// Validate `product_group_data` against the appropriate JSON Schema and any
/// product group-specific cross-field rules (e.g. fibre composition sum).
///
/// The JSON-Schema step resolves against the crate's **embedded** schema
/// registry and catalog (built once at first use). Schemas registered at
/// runtime into a separate [`VersionedSchemaRegistry`] are not visible here —
/// validate those through that registry directly (its fail-closed
/// `validate_strict`).
///
/// A product group with no typed variant in this build is still validated against its
/// embedded schema if the catalog has one. It is a **hard error** only when the
/// product group is unknown to the catalog too — pass a [`ProductGroupValidatorRegistry`]
/// via [`validate_product_group_data_with_registry`] to handle product groups added after
/// this crate was released.
///
/// # Errors
///
/// Returns `ValidationErrors` listing every failing field when validation
/// fails. The `Ok(())` path means the data is structurally valid.
pub fn validate_product_group_data(
    product_group_data: &ProductGroupData,
) -> Result<(), ValidationErrors> {
    validate_product_group_data_with_registry(
        product_group_data,
        &ProductGroupValidatorRegistry::default(),
    )
}

/// Like [`validate_product_group_data`] but accepts a runtime validator registry.
///
/// For an untyped product group, validates against the embedded schema for **that
/// product group's own key** when the catalog has one, and additionally against any
/// validator registered under the same key in `registry`. The key is preserved
/// through deserialization, so dispatch is by name rather than by a literal
/// `"other"`.
///
/// A hard error only when there is **neither** — a product group this build has never
/// heard of, with nothing to check it against.
pub fn validate_product_group_data_with_registry(
    product_group_data: &ProductGroupData,
    registry: &ProductGroupValidatorRegistry,
) -> Result<(), ValidationErrors> {
    let mut errors: Vec<FieldError> = Vec::new();
    if let ProductGroupData::Other { product_group, .. } = product_group_data {
        // Dispatch by the product group's own key, not by a literal "other": the key
        // survives deserialization precisely so a product group with no typed variant
        // in this build can still be handled by name.
        //
        // Delegated to `validate_raw_product_group_data` rather than going straight to
        // `registry`, so an untyped product group still gets the **embedded** schema
        // when the catalog has one. Those two facts are independent: a schema
        // ships as a data file, a typed variant is Rust code, and a product group can
        // have the first without the second. Checking only `registry` rejected
        // every such product group outright — which is the opposite of what the open
        // product group lane is for, and would have turned removing a typed lane into
        // a hard regression for any product group whose schema we already ship.
        if let Err(ve) = validate_raw_product_group_data(
            product_group,
            &product_group_data_instance(product_group_data),
            registry,
        ) {
            errors.extend(ve.errors);
        }
    } else {
        schema_errors(product_group_data, &mut errors);
        cross_field_errors(product_group_data, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrors { errors })
    }
}

/// Validate raw product group JSON using the embedded schema registry and any
/// runtime cross-field validator.
///
/// This is the extension point for the plugin host: when a plugin produces a
/// DPP with a product group key not present in the compile-time `ProductGroupData` enum,
/// pass the raw JSON through this function with an appropriate
/// `ProductGroupValidatorRegistry`.
///
/// Validation steps:
/// 1. JSON Schema — resolved via the embedded [`ProductGroupCatalog`] and
///    [`VersionedSchemaRegistry`] (for product groups with a registered schema).
/// 2. Cross-field — dispatched to `registry.get(product_group_key)` when present.
/// 3. Hard error — if neither a schema nor a registered validator exists for
///    `product_group_key`.
pub fn validate_raw_product_group_data(
    product_group_key: &str,
    data: &serde_json::Value,
    registry: &ProductGroupValidatorRegistry,
) -> Result<(), ValidationErrors> {
    let mut errors: Vec<FieldError> = Vec::new();
    let catalog = default_catalog();
    let has_schema = catalog.current_schema_version(product_group_key).is_some();

    if let Some(version_str) = catalog.current_schema_version(product_group_key) {
        match version_str.parse::<semver::Version>() {
            Ok(version) => {
                if let Err(ve) = default_registry().validate(product_group_key, &version, data) {
                    errors.extend(ve.errors);
                }
            }
            // Fail closed: a registered product group with an unparseable current
            // version must not silently skip schema validation.
            Err(_) => errors.push(FieldError {
                field: "/schemaVersion".to_owned(),
                message: format!(
                    "product_group '{product_group_key}' has an invalid current schema version '{version_str}'"
                ),
            }),
        }
    }

    match registry.get(product_group_key) {
        Some(v) => {
            if let Err(field_errors) = v.validate(data) {
                errors.extend(field_errors);
            }
        }
        None if !has_schema => {
            errors.push(FieldError {
                field: "/product_group".to_owned(),
                message: format!(
                    "unknown product_group \"{product_group_key}\": no JSON schema or cross-field validator registered"
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
fn schema_errors(product_group_data: &ProductGroupData, errors: &mut Vec<FieldError>) {
    let product_group = product_group_data.product_group();
    let key = product_group.catalog_key();
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
                    "product_group '{key}' has an invalid current schema version '{version_str}'"
                ),
            });
            return;
        }
    };
    let instance = product_group_data_instance(product_group_data);
    if let Err(ve) = default_registry().validate(key, &version, &instance) {
        errors.extend(ve.errors);
    }
}

/// The JSON the schema expects: the inner product group fields without the `"productGroup"`
/// discriminant tag that `ProductGroupData` serialises (schemas forbid extra props).
fn product_group_data_instance(product_group_data: &ProductGroupData) -> serde_json::Value {
    let mut value =
        serde_json::to_value(product_group_data).expect("ProductGroupData serializes to Value");
    if let Some(obj) = value.as_object_mut() {
        obj.remove("productGroup");
    }
    value
}

/// Cross-field regulatory rules that JSON Schema cannot express, delegated to
/// `dpp-rules` through the `dpp-domain` adapters.
fn cross_field_errors(product_group_data: &ProductGroupData, errors: &mut Vec<FieldError>) {
    match product_group_data {
        ProductGroupData::Battery(d) => {
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
            let chemistry = d.battery_chemistry.wire_str();
            for metal in battery_recycled_chemistry_conflicts(
                chemistry,
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
        ProductGroupData::Textile(d) => {
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
        ProductGroupData::Electronics(d) => push_svhc(d.svhc_substances.as_deref(), errors),
        ProductGroupData::Toy(d) => push_svhc(d.svhc_substances.as_deref(), errors),
        ProductGroupData::Furniture(d) => push_svhc(d.svhc_substances.as_deref(), errors),
        ProductGroupData::Mattress(d) => push_svhc(d.svhc_substances.as_deref(), errors),
        ProductGroupData::Detergent(d) => {
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

/// Validate a passport completely: its own invariants, then schema conformance
/// of its product-group data.
///
/// This is the pairing [`Passport::validate`] used to do alone, split because
/// the two halves are not the same kind of check.
///
/// [`Passport::validate`] is the aggregate stating what must be true of itself —
/// a non-empty product name, a well-formed schema version, a repairability score
/// inside its range. It needs nothing but the record, so it runs on every target
/// and never fails for a reason outside the passport.
///
/// Schema conformance needs the versioned registry, which reaches `jsonschema`
/// and through it a blocking HTTP client. An aggregate that cannot state its own
/// invariants without a network stack in its dependency tree is the wrong shape,
/// and the `#[cfg(not(target_arch = "wasm32"))]` that used to sit inside
/// `validate` was that fact showing through as a conditional rather than as a
/// boundary.
///
/// Callers wanting both — the publish path — want this. Callers wanting only the
/// record's own consistency want [`Passport::validate`].
#[must_use = "a validation result that is discarded has validated nothing"]
pub fn validate_passport(passport: &Passport) -> Result<(), DppError> {
    let mut errors = match passport.validate() {
        Ok(()) => Vec::new(),
        Err(DppError::Validation(ve)) => ve.errors,
        Err(other) => return Err(other),
    };

    if let Some(ref data) = passport.product_group_data
        && let Err(ve) = validate_product_group_data(data)
    {
        errors.extend(ve.errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(DppError::Validation(ValidationErrors { errors }))
    }
}
