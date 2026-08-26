//! Bill-of-materials graph construction and traversal.

use super::bom::*;
use crate::passport::PassportId;

fn id() -> PassportId {
    PassportId::new()
}

#[test]
fn independent_child_is_accepted() {
    let edges = ComponentEdges::new();
    let (parent, child) = (id(), id());
    assert_eq!(check_edge(&edges, parent, child, DEFAULT_DEPTH_CAP), Ok(()));
}

#[test]
fn self_edge_is_a_cycle() {
    let edges = ComponentEdges::new();
    let p = id();
    assert_eq!(
        check_edge(&edges, p, p, DEFAULT_DEPTH_CAP),
        Err(EdgeRejection::Cycle)
    );
}

#[test]
fn direct_back_edge_is_a_cycle() {
    // child already lists parent as one of its components.
    let (parent, child) = (id(), id());
    let mut edges = ComponentEdges::new();
    edges.insert(child, vec![parent]);
    assert_eq!(
        check_edge(&edges, parent, child, DEFAULT_DEPTH_CAP),
        Err(EdgeRejection::Cycle)
    );
}

#[test]
fn transitive_back_edge_is_a_cycle() {
    // child → mid → parent : adding parent → child closes the loop.
    let (parent, child, mid) = (id(), id(), id());
    let mut edges = ComponentEdges::new();
    edges.insert(child, vec![mid]);
    edges.insert(mid, vec![parent]);
    assert_eq!(
        check_edge(&edges, parent, child, DEFAULT_DEPTH_CAP),
        Err(EdgeRejection::Cycle)
    );
}

#[test]
fn shared_subcomponent_diamond_is_not_a_cycle() {
    // child → {a, b}, and both a and b → leaf. A diamond, not a cycle.
    let (parent, child, a, b, leaf) = (id(), id(), id(), id(), id());
    let mut edges = ComponentEdges::new();
    edges.insert(child, vec![a, b]);
    edges.insert(a, vec![leaf]);
    edges.insert(b, vec![leaf]);
    assert_eq!(check_edge(&edges, parent, child, DEFAULT_DEPTH_CAP), Ok(()));
}

#[test]
fn subtree_deeper_than_cap_is_refused() {
    // A straight chain child → n1 → n2 … deeper than the cap.
    let parent = id();
    let chain: Vec<PassportId> = (0..8).map(|_| id()).collect();
    let mut edges = ComponentEdges::new();
    for pair in chain.windows(2) {
        edges.insert(pair[0], vec![pair[1]]);
    }
    assert_eq!(
        check_edge(&edges, parent, chain[0], 3),
        Err(EdgeRejection::DepthExceeded)
    );
    // A cap that comfortably covers the chain accepts it.
    assert_eq!(check_edge(&edges, parent, chain[0], 32), Ok(()));
}

#[test]
fn pre_existing_cycle_in_edges_still_terminates() {
    // The adjacency itself already contains a loop (x → y → x). The check
    // must terminate via the visited set rather than spin forever; since the
    // unrelated `parent` is not reachable from the loop, it is safe to add.
    let (parent, x, y) = (id(), id(), id());
    let mut edges = ComponentEdges::new();
    edges.insert(x, vec![y]);
    edges.insert(y, vec![x]);
    assert_eq!(check_edge(&edges, parent, x, 32), Ok(()));
}
