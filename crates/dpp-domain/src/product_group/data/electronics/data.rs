//! Electronics — EU Reg. 2023/1670 (ecodesign) + 2023/1669 (energy labelling).
//!
//! Applicability dates and regulatory status live in the product group manifest
//! (`product-groups/electronics.json`); see `docs/regulatory/REGULATORY.md` for what is
//! implemented versus pending.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::index_scope_exclusion::IndexScopeExclusion;
use super::repairability_index::RepairabilityIndexDeclaration;
use crate::identifier::ProductIdentifier;
use crate::product_group::repairability_score::RepairabilityScore;
use crate::product_group::{DeviceType, EnergyEfficiencyClass};

use super::super::common::{CriticalRawMaterial, SvhcSubstance};

/// Electronics product group data.
///
/// Scope and applicability dates are held in the product group manifest
/// (`product-groups/electronics.json`), not restated here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElectronicsData {
    /// The unique product identifier for the product model, in whichever EN 18219
    /// clause 5 scheme issued it — a 14-digit GTIN under scheme 1.
    pub product_identifier: ProductIdentifier,
    /// Device type per EU Regulation (EU) 2023/1670 Art. 1(1).
    pub product_category: DeviceType,
    /// EU energy label class (A–G) per Energy Labelling Regulation 2017/1369.
    pub energy_efficiency_class: EnergyEfficiencyClass,
    /// Whole-lifecycle carbon footprint in kg CO₂e per unit.
    pub co2e_per_unit_kg: f64,

    /// Repairability score (non-regulatory heuristic — not EN 45554 / EU 2023/1669).
    /// `overall` ≥ 6.0 = good; < 4.0 = fails minimum standard.
    ///
    /// **Not comparable to the enacted index.** Where both are present they are
    /// two different numbers on two different scales, and only
    /// [`repairability_index_inputs`](Self::repairability_index_inputs) feeds the
    /// one Reg. (EU) 2023/1669 defines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repairability_score: Option<RepairabilityScore>,
    /// The Annex IV point 5 parameters an operator declares, from which the
    /// enacted repairability index of Reg. (EU) 2023/1669 is computed.
    ///
    /// ✅ COMPLIANCE-PIN: EU 2023/1669, Annex IV point 5 (OJ L 214, 31.8.2023,
    /// p. 26). Declared values, not a score — see
    /// [`RepairabilityIndexDeclaration`] for why the two are kept apart, and for
    /// the Annex IX Table 10 verification tolerance that makes the split the
    /// same shape the Regulation itself describes.
    ///
    /// `None` is the ordinary case and always will be for a device type the act
    /// does not reach. **Nothing obliges a passport to carry this** — the word
    /// "passport" does not occur in Reg. (EU) 2023/1669, which puts the index on
    /// the energy label and in the product information sheet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repairability_index_inputs: Option<RepairabilityIndexDeclaration>,
    /// A declaration that Art. 1(a) or Art. 1(b) of Reg. (EU) 2023/1669 carves
    /// this unit out of the index's scope.
    ///
    /// ✅ COMPLIANCE-PIN: EU 2023/1669, Art. 1 (OJ L 214, 31.8.2023, p. 12). A
    /// rollable-display phone and a high-security smartphone are both
    /// [`DeviceType::Smartphone`], so without this the index would be claimed
    /// over two product classes the Regulation expressly disclaims.
    ///
    /// `None` means **not excluded**, never "unknown": the carve-out is what
    /// removes the obligation, so an operator who declares nothing has claimed
    /// nothing, and reading silence as an exclusion would exempt a product on a
    /// missing field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_scope_exclusion: Option<IndexScopeExclusion>,
    /// Whether spare parts are commercially available from the manufacturer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare_parts_available: Option<bool>,
    /// URL to the repair manual or repair information portal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_manual_url: Option<String>,
    /// URL to disassembly / dismantling instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disassembly_instructions_url: Option<String>,
    /// SVHC substances present above 0.1% w/w (REACH Art. 33).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svhc_substances: Option<Vec<SvhcSubstance>>,
    /// Whether the product complies with RoHS Directive 2011/65/EU.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rohs_compliant: Option<bool>,
    /// Critical raw materials present (EU CRM Act 2024/1252).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_raw_materials: Option<Vec<CriticalRawMaterial>>,
    /// Recycled content as a percentage of total product weight (0.0–100.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_content_pct: Option<f64>,
    /// Standby power consumption in watts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standby_power_w: Option<f64>,
    /// Expected product lifetime in years under normal use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime_years: Option<u32>,
    /// Date until which firmware / software updates are guaranteed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_update_until: Option<DateTime<Utc>>,
}

impl crate::product_group::payload::ProductGroupPayload for ElectronicsData {
    fn product_identifier(&self) -> Option<&crate::identifier::ProductIdentifier> {
        Some(&self.product_identifier)
    }

    /// This act defines no model identifier.
    fn model_identifier(&self) -> Option<&str> {
        None
    }

    /// This group's schema carries a REACH candidate-list declaration.
    fn svhc_substances(&self) -> Option<&[crate::product_group::SvhcSubstance]> {
        self.svhc_substances.as_deref()
    }

    /// Reg. (EU) 2023/1670 Art. 1(1)'s four device types. Always present: the
    /// field is required and the enum is closed.
    fn product_category(&self) -> Option<&str> {
        Some(self.product_category.wire_str())
    }
}
