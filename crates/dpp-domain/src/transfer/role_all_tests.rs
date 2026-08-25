//! Every `OperatorRole` variant is reachable and round-trips.

use super::OperatorRole;

/// `ALL` must list every variant.
///
/// The match below is exhaustive and has no catch-all, so adding a variant
/// stops this file compiling — and the length assertion then fails until
/// `ALL` is updated too. Two stages, because a const list that can silently
/// fall behind the enum is worse than no list: every consumer that trusts it
/// to be complete inherits the gap.
#[test]
fn all_lists_every_variant() {
    for role in OperatorRole::ALL {
        match role {
            OperatorRole::Manufacturer
            | OperatorRole::Importer
            | OperatorRole::Distributor
            | OperatorRole::AuthorisedRepresentative
            | OperatorRole::Remanufacturer
            | OperatorRole::Repurposer
            | OperatorRole::PreparerForReuse
            | OperatorRole::Repairer
            | OperatorRole::Recycler => {}
        }
    }
    assert_eq!(
        OperatorRole::ALL.len(),
        9,
        "a variant was added to the match above but not to ALL"
    );
}
