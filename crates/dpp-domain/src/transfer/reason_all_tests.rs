//! Every `TransferReason` variant is reachable and round-trips.

use super::TransferReason;

/// `ALL` must list every variant — see `OperatorRole`'s equivalent test for
/// why this is two stages rather than one.
#[test]
fn all_lists_every_variant() {
    for reason in TransferReason::ALL {
        match reason {
            TransferReason::Sale
            | TransferReason::Return
            | TransferReason::Remanufacturing
            | TransferReason::Repurposing
            | TransferReason::PreparationForReuse
            | TransferReason::Import
            | TransferReason::InsolvencySuccession => {}
        }
    }
    assert_eq!(
        TransferReason::ALL.len(),
        7,
        "a variant was added to the match above but not to ALL"
    );
}
