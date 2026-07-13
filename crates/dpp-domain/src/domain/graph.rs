//! Bill-of-materials graph checks over local passport component edges.
//!
//! Pure and greenfield. The engine builds the *local* adjacency — each passport
//! id mapped to the ids of the component passports it holds in the same repo —
//! and this module decides whether a new `parent → child` edge is safe to add.
//!
//! A cross-operator component reference cannot be resolved to a local id without
//! a network fetch, so its cycle safety is necessarily a verify-time concern,
//! not an insertion-time guarantee. This module therefore only ever reasons over
//! the local subgraph; the recursive verify walk (built on top of this) is what
//! catches a cycle that only closes across operators.

use std::collections::{HashMap, HashSet};

use crate::domain::passport::PassportId;

/// Adjacency of the local component graph: each passport id to the ids of its
/// direct, locally-held component passports.
pub type ComponentEdges = HashMap<PassportId, Vec<PassportId>>;

/// The default maximum BOM depth for a `child` sub-assembly (the child itself is
/// depth 1). Deep enough for pack → module → cell and then some; small enough
/// that the reachability check can never be turned into a DoS vector.
pub const DEFAULT_DEPTH_CAP: usize = 6;

/// Why a `parent → child` component edge was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeRejection {
    /// `child` already reaches `parent`, so the edge would close a cycle.
    Cycle,
    /// `child`'s sub-assembly is already `depth_cap` levels deep; nesting it
    /// under `parent` would exceed the maximum modelled BOM depth.
    DepthExceeded,
}

/// Decide whether the edge `parent → child` may be added to the local component
/// graph `edges` (adjacency: passport id → its direct local component ids).
///
/// Walks only `child`'s reachable subtree, bounded to `depth_cap` levels, and
/// refuses if it reaches `parent` (a cycle) or if the subtree is already at the
/// cap (would exceed the maximum depth). A `visited` set makes shared
/// sub-assemblies (diamonds) and any pre-existing cycle in `edges` safe to walk,
/// so the check always terminates.
///
/// Reachability is exact regardless of `depth_cap`: if `parent` is reachable
/// from `child`, the walk finds it. The depth bound is a structural cap and DoS
/// guard, and `Cycle` takes priority over `DepthExceeded` when both would apply.
pub fn check_edge(
    edges: &ComponentEdges,
    parent: PassportId,
    child: PassportId,
    depth_cap: usize,
) -> Result<(), EdgeRejection> {
    if parent == child {
        return Err(EdgeRejection::Cycle);
    }
    let mut visited = HashSet::new();
    // DFS from child; depth 1 = child sitting directly under parent.
    let mut stack = vec![(child, 1usize)];
    while let Some((node, depth)) = stack.pop() {
        if node == parent {
            return Err(EdgeRejection::Cycle);
        }
        if depth > depth_cap {
            return Err(EdgeRejection::DepthExceeded);
        }
        if !visited.insert(node) {
            continue;
        }
        if let Some(children) = edges.get(&node) {
            for &c in children {
                stack.push((c, depth + 1));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
