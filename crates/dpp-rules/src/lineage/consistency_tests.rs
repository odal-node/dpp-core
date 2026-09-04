//! The life-status consistency rule, including the cases it deliberately allows.

use super::consistency::check_life_status_consistency;
use super::status_defect::StatusDefect;

#[test]
fn a_status_its_edges_support_is_consistent() {
    assert_eq!(
        check_life_status_consistency(Some("remanufactured"), &["remanufacturing"]),
        None
    );
    assert_eq!(
        check_life_status_consistency(Some("re-used"), &["preparationForReuse"]),
        None
    );
}

/// Both Art. 3(30) and Art. 3(31) yield a repurposed unit.
///
/// The two differ by the waste status of the *input*, not the outcome, so a
/// passport claiming `repurposed` is supported by either edge. Requiring
/// `repurposing` alone would reject a lawful record produced from a waste
/// battery.
#[test]
fn either_repurposing_operation_supports_repurposed() {
    for operation in ["repurposing", "preparationForRepurposing"] {
        assert_eq!(
            check_life_status_consistency(Some("repurposed"), &[operation]),
            None,
            "{operation} must support a repurposed unit"
        );
    }
}

#[test]
fn a_status_no_edge_supports_is_reported() {
    assert_eq!(
        check_life_status_consistency(Some("remanufactured"), &["repurposing"]),
        Some(StatusDefect::NoEdgeSupportsStatus {
            status: "remanufactured"
        })
    );
}

/// A second-life claim with no lineage at all is the same defect.
///
/// The operation the status asserts is unrecorded either way — whether the edges
/// name something else or there are none.
#[test]
fn a_second_life_status_with_no_edges_is_reported() {
    assert_eq!(
        check_life_status_consistency(Some("repurposed"), &[]),
        Some(StatusDefect::NoEdgeSupportsStatus {
            status: "repurposed"
        })
    );
}

/// One edge is enough, and that is deliberate.
///
/// Art. 77(7) permits several predecessors and nothing forces them to share an
/// operation, so a unit built from one repurposed and one remanufactured
/// predecessor is lawful and has no unambiguous derived status. Requiring
/// *every* edge to agree would make that record unrepresentable — which is the
/// exact failure that makes deriving the status instead of storing it a bad
/// idea.
#[test]
fn mixed_predecessors_are_consistent_with_either_status() {
    let edges = ["repurposing", "remanufacturing"];
    assert_eq!(
        check_life_status_consistency(Some("repurposed"), &edges),
        None
    );
    assert_eq!(
        check_life_status_consistency(Some("remanufactured"), &edges),
        None
    );
}

#[test]
fn original_with_a_derivation_edge_is_reported() {
    assert_eq!(
        check_life_status_consistency(Some("original"), &["repurposing"]),
        Some(StatusDefect::OriginalIsDerived {
            operation: "repurposing"
        })
    );
}

#[test]
fn original_with_no_edges_is_consistent() {
    assert_eq!(check_life_status_consistency(Some("original"), &[]), None);
}

/// `waste` is a transition on a record that continues, so its edges are silent
/// about it.
///
/// Art. 77(7)'s second subparagraph moves responsibility on a battery becoming
/// waste and mandates **no new passport**. A repurposed unit that later became
/// waste therefore carries a `repurposing` edge and a `waste` status, and both
/// are correct. Checking the status against the edges would report that entirely
/// ordinary record as inconsistent — including the case below, where the edges
/// would otherwise contradict every other status.
#[test]
fn waste_is_consistent_with_any_lineage() {
    for edges in [
        &[][..],
        &["repurposing"][..],
        &["remanufacturing", "preparationForReuse"][..],
    ] {
        assert_eq!(
            check_life_status_consistency(Some("waste"), edges),
            None,
            "waste must not be checked against {edges:?}"
        );
    }
}

/// No status is not a defect.
///
/// The vocabulary is Reg. (EU) 2023/1542's. A product group whose instrument does
/// not ask the question should not be made to answer it, so an absent status is
/// consistent with any lineage.
#[test]
fn an_absent_status_is_consistent() {
    assert_eq!(check_life_status_consistency(None, &["repurposing"]), None);
    assert_eq!(check_life_status_consistency(None, &[]), None);
}

/// A value outside point 4(c) is reported rather than treated as consistent.
///
/// Unreachable from core, whose `LifeStatus` is closed, but the Wasm plugins hand
/// this crate JSON string fields. "approaching end of life" is the specific
/// invention worth naming: an earlier draft of the design note carried it, and it
/// appears nowhere in the Regulation.
#[test]
fn a_status_outside_the_annex_is_reported() {
    for status in ["approaching end of life", "reused", "Original", ""] {
        assert_eq!(
            check_life_status_consistency(Some(status), &["repurposing"]),
            Some(StatusDefect::UnknownStatus { status }),
            "{status:?} is not one of the five values point 4(c) enumerates"
        );
    }
}
