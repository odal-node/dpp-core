//! The Art. 47 declaration's wire spelling.

use super::*;
use dpp_rules::batteries::due_diligence::OperatorScope;

/// The JSON spelling and the rules crate's wire spelling are one spelling, so
/// the plugin, which reads JSON, and the gate, which reads this type, cannot
/// disagree about a record.
#[test]
fn the_serde_spelling_is_the_rules_spelling() {
    for scope in [
        DueDiligenceOperatorScope::Subject,
        DueDiligenceOperatorScope::BelowTurnoverThreshold,
    ] {
        let wire = serde_json::to_value(scope).unwrap();
        assert_eq!(
            wire.as_str().and_then(OperatorScope::from_wire),
            Some(OperatorScope::from(scope))
        );
    }
}
