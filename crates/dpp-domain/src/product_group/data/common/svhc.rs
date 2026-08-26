//! [`SvhcSubstance`] — a substance of very high concern under REACH / ECHA SCIP.

use serde::{Deserialize, Serialize};

/// A substance of very high concern (SVHC) declared under REACH / ECHA SCIP database.
///
/// ESPR requires textile DPPs to disclose any SVHC present above 0.1% w/w in the
/// article, linking to the ECHA SCIP database entry where applicable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SvhcSubstance {
    /// CAS Registry Number, e.g. `"80-05-7"` (Bisphenol A).
    pub cas_number: String,
    /// Human-readable substance name, e.g. `"Bisphenol A"`.
    pub substance_name: String,
    /// Concentration in the article as weight-% (0.0–100.0).
    /// The SVHC threshold under REACH Article 33 is 0.1% w/w.
    pub concentration_pct: f64,
    /// Where in the product the substance is found, e.g. `"coating"`, `"dye"`, `"finish"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_in_product: Option<String>,
    /// ECHA SCIP database notification reference, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scip_notification_id: Option<String>,
}
