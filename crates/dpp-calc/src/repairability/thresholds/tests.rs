//! Cross-ruleset metadata and effective-date-guard tests for the stub rulesets.

use super::*;
use crate::clock::AssessmentClock;
use crate::error::CalcError;
use crate::repairability::calculate;
use crate::repairability::parameters::RepairabilityInputs;
use crate::ruleset::Effectivity;

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
        // These acts are Pending, so they govern no date at all.
        assert!(matches!(rs.effectivity(), Effectivity::Pending { .. }));
    }
}

#[test]
fn calculating_with_a_pending_ruleset_is_rejected_as_undetermined() {
    // Laptop/Displays/Washing all await an unadopted ESPR delegated act, so
    // calculate() must refuse them — and as *undetermined*, not "not yet
    // effective". There is no date to be waiting for.
    let clock =
        AssessmentClock::placed_on(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"));
    for result in [
        calculate(&valid_inputs(), &LaptopRuleset, clock),
        calculate(&valid_inputs(), &DisplaysRuleset, clock),
        calculate(&valid_inputs(), &WashingMachineRuleset, clock),
    ] {
        assert!(matches!(result, Err(CalcError::RulesetUndetermined { .. })));
    }
}
