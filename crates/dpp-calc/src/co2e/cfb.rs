//! Battery Carbon Footprint (CFB) calculator — Phase 2 placeholder.
//!
//! Will implement Article 7 of EU Battery Regulation 2023/1542 once the carbon
//! footprint delegated act under that article is adopted. **It has not been.**
//! It was due February 2025 and has slipped repeatedly; as of July 2026 no OJ
//! number exists. The methodology is expected to follow the PEF Category Rules
//! (PEF-CR) for rechargeable batteries.
//!
//! Do not mistake Commission Delegated Regulation (EU) 2025/606 for this act —
//! that one covers recycling efficiency and material recovery rates, a different
//! subject, and is not a basis for carbon footprint calculation.
//!
//! **Status: stub.** This module compiles but all entry points return
//! `CalcError::NotImplemented`. Implementation is gated on Phase 1 completion:
//!   1. Signed ecoinvent / EF dataset reseller sublicense.
//!   2. Legal warranty scope agreed with counsel.
//!   3. Reference CFB vectors extracted from the notified-body test report.
//!
//! ⚠️ COMPLIANCE-PIN PENDING: CFB Delegated Act number ("EU 2025/…") must be
//! confirmed against EUR-Lex before implementation begins. Cite the final OJ number.

use crate::error::CalcError;
use crate::factor::FactorProvider;
use crate::receipt::CalculationReceipt;
use crate::ruleset::Ruleset;
use serde::{Deserialize, Serialize};

/// Regulatory ruleset for the Battery Carbon Footprint methodology.
///
/// Extends [`Ruleset`] — concrete impls will carry the CFB-specific parameters
/// (allocation rules, system boundary, performance class thresholds per EU 2023/1542
/// Delegated Act). No concrete impls exist yet — gated on Phase 1 (signed factor-data
/// license + confirmed delegated act number).
pub trait CfbRuleset: Ruleset {
    // Battery Regulation 2023/1542, Art. 7, CFB Delegated Act (March 2025 draft).
    // Parameters TBD from the delegated act's Annex; to be added in Phase 2.
}

/// Inputs for the battery CFB calculation per EU 2023/1542 Art. 7.
///
/// Field names follow the Annex XIII data attribute names from the regulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfbInputs {
    /// Battery model identifier (used in the receipt for traceability).
    pub model_id: String,
    /// Battery chemistry family (e.g. "LFP", "NMC", "NCA") — determines process
    /// route. Family-level per `BatteryChemistry`; sub-ratios (e.g. "NMC811") are
    /// composition data carried by the cathode/anode material attributes, not here.
    pub battery_chemistry: String,
    /// Declared nominal capacity in kWh.
    pub nominal_capacity_kwh: f64,
    // Additional life-cycle-stage inputs will be added when the Phase 1 data
    // license is in place and the exact CFB methodology steps are confirmed.
}

/// Output of the CFB calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfbResult {
    /// Total cradle-to-grave carbon footprint in kg CO₂e per kWh of capacity.
    pub cfb_kg_co2e_per_kwh: f64,
    /// Absolute carbon footprint of this battery unit in kg CO₂e.
    pub cfb_kg_co2e_total: f64,
    /// Proof-of-calculation receipt (includes factor dataset version).
    pub receipt: CalculationReceipt,
}

/// Calculate the CFB for one battery per EU 2023/1542 Art. 7.
///
/// Currently returns `Err(CalcError::NotImplemented)` — implementation is
/// gated on a signed ecoinvent/EF data sublicense. See module-level docs.
pub fn calculate_cfb(
    _inputs: &CfbInputs,
    _provider: &dyn FactorProvider,
) -> Result<CfbResult, CalcError> {
    Err(CalcError::NotImplemented {
        methodology: "battery-cfb".into(),
        reason: "gate: signed ecoinvent/EF sublicense + legal warranty + confirmed \
                 CFB Delegated Act number (EU 2025/…)"
            .into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor::SyntheticFactorProvider;

    #[test]
    fn calculate_cfb_returns_not_implemented() {
        let inputs = CfbInputs {
            model_id: "test-battery".into(),
            battery_chemistry: "NMC".into(),
            nominal_capacity_kwh: 60.0,
        };
        let provider = SyntheticFactorProvider::new(std::iter::empty::<(String, f64)>());
        let err = calculate_cfb(&inputs, &provider).unwrap_err();
        assert!(
            matches!(err, CalcError::NotImplemented { .. }),
            "expected NotImplemented, got {err}"
        );
    }
}
