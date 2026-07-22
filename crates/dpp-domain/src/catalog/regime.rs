//! [`Regime`] — which EU legal instrument a sector's DPP obligation derives from.

use serde::{Deserialize, Serialize};

/// The legal instrument family a sector belongs to.
///
/// Distinct from [`crate::catalog::RegulatoryStatus`], which says *whether* a
/// sector's obligations bind. This says *which law* they come from. The two are
/// independent: a sector can be `Provisional` under ESPR and another
/// `Provisional` under the PPWR, and the determination gate must treat them
/// identically while the catalog still records where each came from.
///
/// ESPR is not the only source of a passport obligation. Of the sectors this
/// crate ships, several derive from standalone instruments — batteries, toys,
/// detergents and construction products each have their own regulation, and
/// electronics rests on the ecodesign and energy-labelling pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Regime {
    /// Ecodesign for Sustainable Products Regulation (EU) 2024/1781, including
    /// the Art. 24/25 unsold-goods obligations and working-plan product groups.
    Espr,
    /// Batteries Regulation (EU) 2023/1542.
    BatteryRegulation,
    /// Toy Safety Regulation (EU) 2025/2509.
    ToySafety,
    /// Detergents Regulation (EU) 2026/405.
    Detergents,
    /// Construction Products Regulation (EU) 2024/3110.
    Cpr,
    /// Packaging and Packaging Waste Regulation (EU) 2025/40.
    Ppwr,
    /// End-of-Life Vehicles Regulation — the Circularity Vehicle Passport.
    Elv,
    /// Product-specific ecodesign and energy-labelling implementing acts, e.g.
    /// Regulations (EU) 2023/1670 and 2023/1669 for smartphones and tablets.
    EcodesignEnergyLabelling,
    /// A named instrument this enum does not yet model.
    Other(String),
    /// Tracked, but no DPP instrument exists. Pairs with
    /// [`crate::catalog::RegulatoryStatus::Watch`].
    None,
}
