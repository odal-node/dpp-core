//! [`ProductCategory`] — a typed product sub-classification within a sector.

use serde::{Deserialize, Serialize};

/// Typed product category — a sub-type *within* a sector.
///
/// **Not** a dispatch key. [`Sector`](crate::domain::sector::Sector) selects
/// the applicable delegated act, schema, and plugin; a `ProductCategory` is a
/// finer classification a plugin may branch on (e.g. battery `ev` vs
/// `portable`, electronics `smartphone`). The list is extensible via `Other`.
/// See `DATA-MODEL.md` §3.5.
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
