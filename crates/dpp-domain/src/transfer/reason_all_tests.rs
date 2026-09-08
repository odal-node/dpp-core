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
            | TransferReason::PreparationForRepurposing
            | TransferReason::WasteHandover
            | TransferReason::Import
            | TransferReason::InsolvencySuccession => {}
        }
    }
    assert_eq!(
        TransferReason::ALL.len(),
        9,
        "a variant was added to the match above but not to ALL"
    );
}

/// All four operations Reg. (EU) 2023/1542 Art. 77(7) names are expressible.
///
/// The article transfers responsibility for "a battery that has been subject to
/// preparation for re-use, preparation for repurposing, repurposing or
/// remanufacturing". Three of the four had variants; `preparation for
/// repurposing` did not, so a transfer performed for that reason could only be
/// recorded as one of the other three — silently mislabelling which operation
/// the OJ text says occurred.
#[test]
fn every_art_77_7_operation_is_expressible() {
    let operations = [
        TransferReason::PreparationForReuse,
        TransferReason::PreparationForRepurposing,
        TransferReason::Repurposing,
        TransferReason::Remanufacturing,
    ];

    for operation in &operations {
        assert!(
            TransferReason::ALL.contains(operation),
            "{operation:?} is an Art. 77(7) operation and must be in ALL"
        );
    }

    let wire: Vec<&str> = operations.iter().map(TransferReason::wire_str).collect();
    assert_eq!(
        wire,
        [
            "preparationForReuse",
            "preparationForRepurposing",
            "repurposing",
            "remanufacturing",
        ],
        "the wire vocabulary a registry receives must name each operation distinctly"
    );
}
