//! The [`Passport`] aggregate root.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    FacilitySnapshot, ManufacturerInfo, MaterialEntry, PassportId, PassportRef, PassportView,
    ProductCategory,
};
use crate::domain::{
    identity::AccessTier,
    lint::LintResult,
    sector::{CarbonFootprint, RepairabilityScore, Sector, SectorData},
    status::PassportStatus,
};
use crate::ports::compliance::ComplianceResult;
use crate::ports::seal::SealedEnvelope;

/// The canonical Digital Product Passport record as defined by EU ESPR.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Passport {
    pub id: PassportId,
    /// Optional batch or lot identifier.
    pub batch_id: Option<String>,
    pub product_name: String,
    /// EU ESPR sector — the delegated-act bucket that selects the applicable
    /// schema and plugin. (Replaces the former misnamed `product_category`
    /// field, which actually held a sector.)
    pub sector: Sector,
    /// Optional typed product category — a sub-type *within* `sector`
    /// (e.g. `Smartphone`, `EvBattery`). Never a dispatch key. See DATA-MODEL §3.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_category: Option<ProductCategory>,
    pub manufacturer: ManufacturerInfo,
    pub materials: Vec<MaterialEntry>,
    /// CO₂ equivalent per unit — manufacturer-supplied or engine-calculated.
    pub co2e_per_unit: Option<CarbonFootprint>,
    /// Repairability score (non-regulatory heuristic — not EN 45554 / EU 2023/1669).
    pub repairability_score: Option<RepairabilityScore>,
    /// The computed compliance determination — status, metrics, binding
    /// `violations` + advisory `warnings`, and (when a calculation ran) a
    /// receipt. Attached by the engine's `apply_compliance` at create/update.
    /// Part of the signed payload and immutable after retention lock. `None`
    /// until a determination is computed (e.g. a sector with no plugin loaded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compliance_result: Option<ComplianceResult>,
    /// Non-binding plausibility findings from the `dpp-rules` lint pack —
    /// arithmetic and physical-plausibility checks distinct from binding
    /// compliance rules. Never gates publish and may be recomputed at any
    /// time after publish (a lint re-check), unlike `compliance_result` —
    /// see the vault's `POST /dpp/{id}/lint` endpoint. `None` until a lint
    /// pass has run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint_result: Option<LintResult>,
    /// Typed, sector-specific DPP data (EU Battery Regulation, Textile DPP, etc.).
    ///
    /// `None` for passports where sector-specific data has not yet been supplied.
    /// Set this field when publishing to ensure regulatory compliance validation.
    pub sector_data: Option<SectorData>,
    pub status: PassportStatus,
    /// The publicly accessible QR code URL for this passport.
    pub qr_code_url: Option<String>,
    /// Compact JWS signature over the **full** canonical passport payload
    /// (Confidential tier — for authenticated, full-passport verification).
    pub jws_signature: Option<String>,
    /// Compact JWS signature over the **public (redacted) view** of this passport
    /// (Public tier). Lets anyone verify the public passport independently — the
    /// resolver checks this on the unauthenticated `/public/dpp/{id}` route.
    /// Set at publish time; `None` for drafts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_jws_signature: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    /// Semantic version of the sector schema used to validate this record.
    pub schema_version: String,
    /// Set to `true` permanently on first publish; never unset thereafter.
    ///
    /// Retention-locked passports must remain publicly accessible for the period
    /// defined in the applicable EU ESPR delegated act (typically 10–15 years after
    /// the product's end of life).
    #[serde(default)]
    pub retention_locked: bool,

    // ── 0.2 data-model fields ──────────────────────────────────────────────
    /// Monotonically increasing version counter. `1` on first publish; increments
    /// each time a new passport version supersedes this one (set on the successor).
    #[serde(default = "default_version")]
    pub version: u32,
    /// The passport ID this record supersedes. `None` for first-version passports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_id: Option<PassportId>,
    /// Cross-operator reference to the predecessor this passport derives from
    /// (second-life successor linkage). `None` unless this record was issued as a
    /// successor citing a source passport held by another operator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_passport_ref: Option<PassportRef>,
    /// Cross-operator references to the constituent passports this product is
    /// assembled from — its bill of materials. Empty for a unit with no modelled
    /// sub-assemblies. The inverse edge of `parent_passport_ref`: `component_refs`
    /// point down to many constituents, `parent_passport_ref` points up to one
    /// predecessor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub component_refs: Vec<PassportRef>,
    /// Deadline by which this record must remain accessible. Confirmed against the
    /// verbatim OJ text (Regulation (EU) 2024/1781): **Art. 9(2)(i)** requires the
    /// delegated act to specify "the period during which the digital product
    /// passport is to remain available, which shall correspond to at least the
    /// expected lifetime of a specific product"; **Art. 11(e)** restates this as an
    /// essential requirement, available "including after an insolvency, a
    /// liquidation or a cessation of activity" of the responsible operator. The
    /// separate back-up-copy obligation (via a DPP service provider) is **Art.
    /// 10(4)**, not the retention period itself.
    /// Computed at publish time from the catalog `retention_years` for the sector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_until: Option<DateTime<Utc>>,
    /// Opaque link to an internal product-template record. Not a legal identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_id: Option<Uuid>,
    /// EORI or national economic-operator identifier for the responsible party.
    /// Confirmed against the verbatim OJ text (Regulation (EU) 2024/1781):
    /// **Annex III, point (k)** is the data-content basis — "the name, contact
    /// details and unique operator identifier of the economic operator established
    /// in the Union responsible for carrying out the tasks set out in Article 4 of
    /// Regulation (EU) 2019/1020 [...]"; the identifier-issuance mechanics are
    /// **Art. 12**. (**Art. 13** governs uploading identifiers to the EU registry —
    /// a related but distinct obligation, not the field's basis.) Populated by the
    /// engine from `operator_config`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_identifier: Option<String>,
    /// Snapshot of the Annex III facility where this product was manufactured or
    /// processed, copied by value at create time. Self-contained so the signed
    /// passport stays a complete record independent of the operator's mutable
    /// facility registry (a retired facility never orphans a published passport).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facility: Option<FacilitySnapshot>,
    /// The eIDAS qualified electronic seal applied to this passport, if any.
    /// `placeholder: true` on the envelope means no legally valid seal exists yet —
    /// consumers must check this flag rather than inferring validity from presence.
    /// `None` until a seal (real or placeholder) has been applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seal: Option<SealedEnvelope>,
}

fn default_version() -> u32 {
    1
}

impl Passport {
    /// Validate passport fields for structural correctness and sector-data integrity.
    ///
    /// Checks:
    /// - `product_name` is non-empty
    /// - `manufacturer.name` is non-empty
    /// - `manufacturer.address` is non-empty
    /// - `schema_version` follows semver pattern (x.y.z)
    /// - `co2e_per_unit` is non-negative if present
    /// - `repairability_score` is in range [0.0, 10.0] if present
    /// - `sector_data.sector()` matches `self.sector` if present
    /// - `sector_data` passes JSON Schema + cross-field rules via
    ///   [`crate::domain::validation::validate_sector_data`] (non-wasm32 only)
    pub fn validate(&self) -> Result<(), crate::domain::error::DppError> {
        use crate::domain::field_error::{FieldError, ValidationErrors};

        let mut errors: Vec<FieldError> = Vec::new();

        if self.product_name.trim().is_empty() {
            errors.push(FieldError {
                field: "/productName".to_owned(),
                message: "product_name must not be empty".to_owned(),
            });
        }
        if self.manufacturer.name.trim().is_empty() {
            errors.push(FieldError {
                field: "/manufacturer/name".to_owned(),
                message: "manufacturer.name must not be empty".to_owned(),
            });
        }
        if self.manufacturer.address.trim().is_empty() {
            errors.push(FieldError {
                field: "/manufacturer/address".to_owned(),
                message: "manufacturer.address must not be empty".to_owned(),
            });
        }

        // Must parse as strict semver (major.minor.patch, optional pre-release
        // / build metadata). A hand-rolled digit check let ".5.0" (empty major)
        // and "1.0.abc" (non-numeric patch) through — both then fail
        // `semver::Version` parsing at schema resolution and silently skip
        // schema validation, so reject them here rather than downstream.
        if self.schema_version.parse::<semver::Version>().is_err() {
            errors.push(FieldError {
                field: "/schemaVersion".to_owned(),
                message: "schema_version must be valid semver (e.g. 1.0.0)".to_owned(),
            });
        }

        if let Some(ref cf) = self.co2e_per_unit
            && cf.value_kg < 0.0
        {
            errors.push(FieldError {
                field: "/co2ePerUnit".to_owned(),
                message: "co2e_per_unit must not be negative".to_owned(),
            });
        }

        if let Some(ref rs) = self.repairability_score
            && !(0.0..=10.0).contains(&rs.overall)
        {
            errors.push(FieldError {
                field: "/repairabilityScore".to_owned(),
                message: "repairability_score must be between 0.0 and 10.0".to_owned(),
            });
        }

        // The declared sector must match the sector of the typed data, if present.
        if let Some(ref data) = self.sector_data
            && data.sector() != self.sector
        {
            errors.push(FieldError {
                field: "/sector".to_owned(),
                message: "sector must match sector_data's sector".to_owned(),
            });
        }

        // Sector-data validation: JSON Schema + cross-field rules (fibre sum, SVHC, etc.).
        // Excluded from wasm32 builds because jsonschema depends on reqwest's blocking API.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref data) = self.sector_data
            && let Err(ve) = crate::domain::validation::validate_sector_data(data)
        {
            errors.extend(ve.errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(crate::domain::error::DppError::Validation(
                ValidationErrors { errors },
            ))
        }
    }

    /// Transition the passport to a new status, enforcing the state machine.
    ///
    /// Valid transitions:
    /// ```text
    /// Draft → Published | Archived
    /// Published → Suspended | Archived
    /// Suspended → Published | Archived
    /// ```
    ///
    /// On the first `Draft → Published` transition this method also:
    /// - Sets `retention_locked = true` (ESPR retention obligation).
    /// - Sets `published_at` to the current timestamp.
    /// - Updates `updated_at`.
    pub fn transition_to(
        &mut self,
        next: PassportStatus,
    ) -> Result<(), crate::domain::error::DppError> {
        if !self.status.can_transition_to(&next) {
            return Err(crate::domain::error::DppError::InvalidTransition {
                current: self.status.to_string(),
                required: next.to_string(),
            });
        }

        let now = chrono::Utc::now();

        // First publish: lock retention and record the timestamp.
        if next == PassportStatus::Published && self.published_at.is_none() {
            self.retention_locked = true;
            self.published_at = Some(now);
        }

        self.status = next;
        self.updated_at = now;
        Ok(())
    }

    /// Return a tier-filtered JSON view of this passport.
    ///
    /// NOTE: this is a self-contained domain convenience, **not** the authoritative
    /// public view. The payload that is signed (`publicJwsSignature`) and served on
    /// the public route is produced by the `dpp-crypto` policy engine (vault
    /// `public_view`), which fails closed on unknown sectors. Do not wire this into
    /// the public-serving path expecting byte-parity with the signed view.
    ///
    /// Fields removed per tier:
    /// - Below `Professional`: `batchId`
    /// - Below `Confidential`: `jwsSignature`, `retentionLocked`
    ///
    /// `sectorData`, when present, is independently redacted via
    /// [`crate::domain::sector::redact_sector_data`] against the sector descriptor
    /// from `catalog`. If the sector is not in the catalog, sector data is
    /// **withheld** from viewers below `Confidential` (fail-closed): without the
    /// descriptor's per-field access tiers the domain layer cannot tell which
    /// fields are safe to expose, so it exposes none. `Confidential` viewers —
    /// who may see every field anyway — still receive the full data.
    pub fn redact(
        &self,
        viewer_tier: AccessTier,
        catalog: &crate::catalog::SectorCatalog,
    ) -> PassportView {
        let mut value = match serde_json::to_value(self) {
            Ok(v) => v,
            Err(_) => return PassportView(serde_json::Value::Null),
        };

        if let Some(obj) = value.as_object_mut() {
            if viewer_tier < AccessTier::Professional {
                obj.remove("batchId");
            }
            if viewer_tier < AccessTier::Confidential {
                obj.remove("jwsSignature");
                obj.remove("retentionLocked");
            }
            // Re-redact sectorData using the catalog's per-field access_tiers.
            if let Some(ref sd) = self.sector_data {
                let key = sd.sector().catalog_key();
                let redacted = if let Some(descriptor) = catalog.get(key) {
                    crate::domain::sector::redact_sector_data(sd, viewer_tier, descriptor)
                } else if viewer_tier >= AccessTier::Confidential {
                    // Unknown sector: the full payload is only safe for the tier
                    // that already sees every field.
                    serde_json::to_value(sd).unwrap_or(serde_json::Value::Null)
                } else {
                    // Fail closed: without per-field tiers we cannot tell which
                    // fields are confidential, so withhold sector data entirely
                    // rather than leak it to a lower tier.
                    serde_json::Value::Null
                };
                obj.insert("sectorData".into(), redacted);
            } else {
                obj.remove("sectorData");
            }
        }

        PassportView(value)
    }
}
