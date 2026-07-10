//! Textile (EU Textile DPP, expected ~Q2 2027 adoption, compliance ~2028–2029).

use serde::{Deserialize, Serialize};

use super::shared::SvhcSubstance;

/// A single fibre entry in a textile product's composition list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FibreEntry {
    /// Fibre name, e.g. `"cotton"`, `"polyester"`, `"recycled_polyester"`.
    pub fibre: String,
    /// Percentage by weight (0.0–100.0).
    ///
    /// All entries in the composition list must sum to approximately 100.0
    /// (± 2.0 percentage point tolerance to accommodate rounding).
    pub pct: f64,
    /// ISO 3166-1 alpha-2 country code where this specific fibre was sourced.
    ///
    /// Per-fibre origin traceability as anticipated by the textile delegated act.
    /// When present, allows downstream actors to verify sourcing per fibre rather
    /// than relying on a single `country_of_raw_material_origin` for the whole product.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_of_origin: Option<String>,
}

/// Textile-specific fields for EU textile DPP compliance.
///
/// Based on the anticipated EU Textile DPP delegated act (~Q2 2027 adoption).
/// Fields marked as `Option` are optional under v1.0.0 but may become mandatory
/// once the delegated act is finalised.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextileData {
    // ── Mandatory fields (v1.0.0) ──────────────────────────────────────────
    /// 14-digit GTIN identifying the textile product.
    pub gtin: String,
    /// List of fibres and their percentage composition. Must sum to ~100%.
    pub fibre_composition: Vec<FibreEntry>,
    /// ISO 3166-1 alpha-2 country code where the textile was manufactured.
    pub country_of_manufacturing: String,
    /// ISO 3758 care symbols or free text care instructions.
    pub care_instructions: String,
    /// Chemical compliance standard, e.g. `"OEKO-TEX 100"`, `"REACH"`, `"GOTS"`.
    pub chemical_compliance_standard: String,

    // ── Environmental metrics ──────────────────────────────────────────────
    /// Total recycled content as a percentage of total weight (0.0–100.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_content_pct: Option<f64>,
    /// Carbon footprint in kg CO₂e per unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbon_footprint_kg_co2e: Option<f64>,
    /// Water consumption in litres per unit produced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water_use_litres: Option<f64>,
    /// Microplastic fibre shedding in milligrams per wash cycle.
    ///
    /// Measured per ISO/DIS 4484 or equivalent. Relevant for synthetic textiles
    /// where microplastic release is a growing regulatory concern.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub microplastic_shedding_mg_per_wash: Option<f64>,

    // ── Durability & repairability ─────────────────────────────────────────
    /// Repairability score (0.0–10.0) per EU methodology.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_score: Option<f64>,
    /// Durability score (0.0–10.0) measuring resistance to pilling,
    /// colour fastness, dimensional stability, and seam strength.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub durability_score: Option<f64>,
    /// Expected number of wash cycles before significant quality degradation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_wash_cycles: Option<u32>,

    // ── Traceability ───────────────────────────────────────────────────────
    /// ISO 3166-1 alpha-2 country of raw material origin (product-level fallback).
    /// Per-fibre origin on `FibreEntry.country_of_origin` takes precedence when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_of_raw_material_origin: Option<String>,

    // ── SCIP / SVHC substance disclosure ───────────────────────────────────
    /// List of SVHC substances present above the 0.1% w/w threshold.
    ///
    /// Required by REACH Article 33 and linked to the ECHA SCIP database.
    /// An empty vec means the manufacturer has checked and found no SVHCs.
    /// `None` means the check has not yet been performed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svhc_substances: Option<Vec<SvhcSubstance>>,

    // ── Substances of concern (beyond SVHC) ──────────────────────────────
    /// Allergens or sensitising substances present in the textile.
    /// Covers contact allergens regulated under REACH Annex XVII entry 72
    /// (e.g. certain disperse dyes, chromium VI, nickel in accessories).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allergens: Option<Vec<String>>,

    /// Restricted substances present that are not classified as SVHC but are
    /// regulated under sector-specific rules (e.g. PFASs, flame retardants,
    /// biocides).  Each entry is a free-text substance identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substances_of_concern: Option<Vec<String>>,

    // ── Circularity & end-of-life ─────────────────────────────────────────
    /// Recyclability classification, e.g. `"mono-material"`, `"multi-material"`,
    /// `"not-recyclable"`.  Based on design-for-recycling assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recyclability_class: Option<String>,

    /// End-of-life instructions: how to dispose of or recycle the product.
    /// Free text or URL pointing to take-back/collection point information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_of_life_instructions: Option<String>,

    /// Whether the product has been refurbished or reused, and if so its
    /// condition grade (e.g. `"like-new"`, `"good"`, `"fair"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reuse_condition: Option<String>,

    /// Number of prior ownership cycles (0 for new products).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_use_cycles: Option<u32>,

    // ── Professional-tier data (gated by access control) ───────────────────
    /// Free-text or structured disassembly / deconstruction instructions.
    /// Gated behind the Professional access tier (Verifiable Credential required).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disassembly_instructions: Option<String>,
    /// Whether spare parts or replacement components are available from the manufacturer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare_parts_available: Option<bool>,
    /// Product net weight in grams (used for per-unit environmental calculations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_weight_grams: Option<f64>,

    // ── Repair & maintenance history ──────────────────────────────────────
    /// URL or structured log of repair events performed on this specific item.
    /// Relevant for products sold with a repair history (e.g. certified
    /// pre-owned programmes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_history_url: Option<String>,

    /// Number of completed professional repairs on this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_count: Option<u32>,

    /// Environmental impact score per garment lifecycle (PEF/OEF methodology).
    /// Dimensionless single score aggregating multiple impact categories.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pef_score: Option<f64>,
}
