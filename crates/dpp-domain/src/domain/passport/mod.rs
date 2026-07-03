//! The `Passport` aggregate root and its unique identifier type.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    identity::AccessTier,
    sector::{CarbonFootprint, RepairabilityScore, Sector, SectorData},
    status::PassportStatus,
};
use crate::ports::compliance::ComplianceResult;
use crate::ports::seal::SealedEnvelope;

#[cfg(test)]
mod tests;

/// Newtype wrapper for a passport's unique identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PassportId(pub Uuid);

impl PassportId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for PassportId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PassportId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Typed product category — a sub-type *within* a sector.
///
/// **Not** a dispatch key. [`Sector`] selects the
/// applicable delegated act, schema, and plugin; a `ProductCategory` is a finer
/// classification a plugin may branch on (e.g. battery `ev` vs `portable`,
/// electronics `smartphone`). The list is extensible via `Other`. See
/// `DATA-MODEL.md` §3.5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProductCategory {
    // Battery
    EvBattery,
    IndustrialBattery,
    LmtBattery,
    // Textile
    Apparel,
    Footwear,
    HomeTextile,
    // Electronics
    Smartphone,
    Laptop,
    Charger,
    // Extensible: any category not yet modelled as a variant.
    Other(String),
}

/// Manufacturer information embedded in the passport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManufacturerInfo {
    pub name: String,
    pub address: String,
    /// The manufacturer's `did:web` URL, e.g. `https://acme.example.com/.well-known/did.json`
    pub did_web_url: Option<String>,
}

/// A self-contained snapshot of the ESPR Annex III facility a passport was
/// stamped with at creation.
///
/// Copied **by value** onto the passport so the published, signed record remains
/// complete for its full retention period even if the operator later retires the
/// source facility from their registry. Field shape matches
/// `dpp_registry::FacilityIdentifier`. See DATA-MODEL §3.3 / ADR-006.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilitySnapshot {
    /// Identifier scheme (e.g. `"gln"`, `"national"`).
    pub scheme: String,
    /// Identifier value (e.g. the 13-digit GLN) — the Annex III unique facility id.
    pub value: String,
    /// Human-readable facility name.
    pub name: String,
    /// ISO 3166-1 alpha-2 country code of the facility.
    pub country: String,
    /// Optional street address / location description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

/// A single material entry in the passport's bill of materials.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialEntry {
    pub name: String,
    pub weight_kg: f64,
    /// Percentage of recycled content (0.0–100.0).
    pub recycled_pct: Option<f64>,
    /// ISO 3166-1 alpha-2 country code of material origin.
    pub origin_country: Option<String>,
}

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

// ─── Tier-filtered view ───────────────────────────────────────────────────────

/// A tier-filtered, serialisable view of a [`Passport`].
///
/// Produced by [`Passport::redact`]. Serialises transparently to JSON —
/// use this type wherever a consumer should only see the fields allowed by
/// their [`AccessTier`].
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct PassportView(pub serde_json::Value);

impl PassportView {
    /// Consume the view and return the underlying JSON value.
    pub fn into_value(self) -> serde_json::Value {
        self.0
    }
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
        let mut issues = Vec::new();

        if self.product_name.trim().is_empty() {
            issues.push("product_name must not be empty");
        }
        if self.manufacturer.name.trim().is_empty() {
            issues.push("manufacturer.name must not be empty");
        }
        if self.manufacturer.address.trim().is_empty() {
            issues.push("manufacturer.address must not be empty");
        }

        // Semver: digits.digits.digits, optionally with pre-release suffix
        let sv = &self.schema_version;
        let parts: Vec<&str> = sv.split('.').collect();
        if parts.len() < 3
            || !parts[0].chars().all(|c| c.is_ascii_digit())
            || !parts[1].chars().all(|c| c.is_ascii_digit())
            || parts[2].is_empty()
        {
            issues.push("schema_version must be valid semver (e.g. 1.0.0)");
        }

        if let Some(ref cf) = self.co2e_per_unit
            && cf.value_kg < 0.0
        {
            issues.push("co2e_per_unit must not be negative");
        }

        if let Some(ref rs) = self.repairability_score
            && !(0.0..=10.0).contains(&rs.overall)
        {
            issues.push("repairability_score must be between 0.0 and 10.0");
        }

        // The declared sector must match the sector of the typed data, if present.
        if let Some(ref data) = self.sector_data
            && data.sector() != self.sector
        {
            issues.push("sector must match sector_data's sector");
        }

        let structural_err = if issues.is_empty() {
            None
        } else {
            Some(issues.join("; "))
        };

        // Sector-data validation: JSON Schema + cross-field rules (fibre sum, SVHC, etc.).
        // Excluded from wasm32 builds because jsonschema depends on reqwest's blocking API.
        #[cfg(not(target_arch = "wasm32"))]
        let sector_err: Option<String> = if let Some(ref data) = self.sector_data {
            crate::domain::validation::validate_sector_data(data)
                .err()
                .map(|ve| ve.to_display())
        } else {
            None
        };
        #[cfg(target_arch = "wasm32")]
        let sector_err: Option<String> = None;

        match (structural_err, sector_err) {
            (None, None) => Ok(()),
            (Some(s), None) | (None, Some(s)) => {
                Err(crate::domain::error::DppError::Validation(s.into()))
            }
            (Some(a), Some(b)) => Err(crate::domain::error::DppError::Validation(
                format!("{a}; {b}").into(),
            )),
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
    /// from `catalog`. If the sector is not in the catalog, sector data is left
    /// unfiltered (fail-open at the domain layer; the vault layer applies
    /// `filter_by_access_tier` as defence-in-depth).
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
                } else {
                    serde_json::to_value(sd).unwrap_or(serde_json::Value::Null)
                };
                obj.insert("sectorData".into(), redacted);
            } else {
                obj.remove("sectorData");
            }
        }

        PassportView(value)
    }
}
