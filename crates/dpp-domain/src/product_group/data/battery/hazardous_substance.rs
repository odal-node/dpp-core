//! [`HazardousSubstance`] — a substance of concern and how it is symbolised.

use serde::{Deserialize, Serialize};

/// A hazardous substance declared under Annex VI Part A point 8.
///
/// Deliberately **not** [`SvhcSubstance`](crate::product_group::SvhcSubstance), which
/// this crate already carries for textile, electronics and furniture. That type
/// is REACH-shaped — a concentration against the Art. 33 threshold, an ECHA
/// SCIP reference — and point 8 is a different instrument naming a different
/// set. Sharing the struct would assert the concepts are the same, which nobody
/// has established.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HazardousSubstance {
    /// Substance name.
    pub name: String,
    /// CAS Registry Number, where the substance has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cas_number: Option<String>,
    /// Concentration in the battery as weight-%, where declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concentration_pct: Option<f64>,
}

/// The chemical symbol Art. 13(5) requires on a battery, where one applies.
///
/// Closed: Art. 13(5) names cadmium and lead and nothing else, and the
/// Commission's guidance records the data point as *"cadmium or lead symbol if
/// applicable"*. "No symbol required" is `Option::None`, which is why there is
/// no variant for it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum HazardSymbol {
    Cadmium,
    Lead,
}
