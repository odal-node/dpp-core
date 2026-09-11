//! Aluminium (EU ESPR ~2030, CBAM-aligned).

use serde::{Deserialize, Serialize};

use crate::identifier::Gtin;
use crate::product_group::ProductionRoute;

/// Aluminium product group data for EU ESPR carbon intensity reporting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AluminiumData {
    /// 14-digit GTIN identifying the aluminium product.
    pub gtin: Gtin,
    /// Alloy series designation, e.g. `"1xxx"`, `"3xxx"`, `"5xxx"`, `"6xxx"`.
    pub alloy_grade: String,
    /// Aluminium production route — determines carbon intensity calculation basis.
    pub production_route: ProductionRoute,
    /// Carbon intensity in kg CO₂e per tonne of aluminium produced.
    pub co2e_per_tonne_kg: f64,
    /// Recycled scrap content as a percentage of total input (0.0–100.0).
    pub recycled_content_pct: f64,
    /// ISO 3166-1 alpha-2 country of primary production.
    pub country_of_origin: String,
    /// Annual production volume in tonnes (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_production_tonnes: Option<f64>,
}

impl crate::product_group::payload::ProductGroupPayload for AluminiumData {
    fn gtin(&self) -> Option<&str> {
        Some(self.gtin.as_str())
    }

    /// This act defines no model identifier.
    fn model_identifier(&self) -> Option<&str> {
        None
    }

    /// This act's schema declares no substances-of-concern field.
    fn svhc_substances(&self) -> Option<&[crate::product_group::SvhcSubstance]> {
        None
    }

    /// This group models no product category.
    ///
    /// The catalog declared `primary`, `secondary-recycled` and `mixed` under
    /// `productCategories` until they were removed, and they were never
    /// categories — they are three of the seven [`ProductionRoute`] variants,
    /// copied from this group's `productionRoute` schema enum. How metal was
    /// made is a different axis from what the product is, and answering with
    /// the route would let a credential scoped to a *category* be satisfied by
    /// a *process*.
    ///
    /// They survived because the catalog-versus-schema drift guard was pointed
    /// at `productionRoute`, so it confirmed the values were legal — of the
    /// wrong property. No act defines an aluminium category axis, so the
    /// catalog now declares none and this answer agrees with it.
    fn product_category(&self) -> Option<&str> {
        None
    }
}
