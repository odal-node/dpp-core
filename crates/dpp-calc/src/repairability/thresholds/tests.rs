//! Cross-ruleset metadata and effective-date-guard tests for the stub rulesets.

use super::*;
use crate::error::CalcError;
use crate::repairability::calculate;
use crate::repairability::parameters::RepairabilityInputs;

fn valid_inputs() -> RepairabilityInputs {
    RepairabilityInputs {
        disassembly: 2,
        spare_parts: 2,
        repair_info: 2,
        diagnostic_tools: 2,
        software_updatability: 2,
        customer_support: 2,
    }
}

#[test]
fn stub_rulesets_expose_consistent_metadata() {
    let rulesets: [&dyn RepairabilityRuleset; 3] =
        [&LaptopRuleset, &DisplaysRuleset, &WashingMachineRuleset];
    for rs in rulesets {
        let w = rs.weights();
        let sum = w.disassembly
            + w.spare_parts
            + w.repair_info
            + w.diagnostic_tools
            + w.software_updatability
            + w.customer_support;
        assert!((sum - 1.0).abs() < 1e-9, "weights must sum to 1.0");
        assert_eq!(rs.thresholds().a, 8.5);
        assert!(!rs.id().0.is_empty());
        assert!(!rs.version().0.is_empty());
        assert!(!rs.regulatory_basis().regulation.is_empty());
        // 2100 sentinel: these acts are not yet in force.
        assert!(
            !rs.effective_dates()
                .is_active_on(chrono::Utc::now().date_naive())
        );
    }
}

#[test]
fn calculating_with_a_not_yet_in_force_ruleset_is_rejected() {
    // Laptop/Displays/Washing all carry the 2100 effective-date sentinel, so
    // calculate() must refuse them — and as *not yet effective*, not "expired"
    // (the from=2100/until=None period has not started, it has not ended).
    for result in [
        calculate(&valid_inputs(), &LaptopRuleset),
        calculate(&valid_inputs(), &DisplaysRuleset),
        calculate(&valid_inputs(), &WashingMachineRuleset),
    ] {
        assert!(matches!(
            result,
            Err(CalcError::RulesetNotYetEffective { .. })
        ));
    }
}
