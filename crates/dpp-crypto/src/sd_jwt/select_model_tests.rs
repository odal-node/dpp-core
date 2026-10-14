//! The claim path itself, the RFC's own choices restated as paths, and a property
//! test against a model (`select.rs`).
//!
//! The RFC's presentations were made by someone else. The model finds the
//! Disclosures a path needs while it builds the token, bottom up, and so shares no
//! code with the walk that finds them afterwards, top down.

use std::collections::BTreeSet;

use proptest::prelude::*;
use serde_json::{Map, Value, json};

use super::rfc9901_examples::{
    A3_ISSUED, A3_PRESENTATION, A3_PROCESSED, S51_ISSUED, S52_PRESENTATION, S52_PROCESSED,
};
use super::select_tests::digest_named;
use super::select_tests::digests_of;
use super::tests::stub_jwt;
use super::{
    ClaimPath, Disclosure, PathSegment, SdJwt, SdJwtError, build_payload, conceal, conceal_elements,
};

// --- the path itself ------------------------------------------------------------

#[test]
fn a_path_in_the_drafts_json_form_is_read_and_written_back() {
    let path = ClaimPath::from_json(&json!(["degrees", null, "type"])).expect("a path");
    assert_eq!(
        path.segments(),
        [
            PathSegment::Key("degrees".to_owned()),
            PathSegment::AllElements,
            PathSegment::Key("type".to_owned()),
        ]
    );
    assert_eq!(path.to_string(), r#"["degrees",null,"type"]"#);

    let with_position = ClaimPath::from_json(&json!(["nationalities", 1])).expect("a path");
    assert_eq!(with_position, ClaimPath::claim("nationalities").index(1));
}

#[test]
fn what_is_not_a_path_is_refused() {
    for not_a_path in [
        json!([]),
        json!("address"),
        json!({ "path": ["a"] }),
        json!(["a", -1]),
        json!(["a", 1.5]),
        json!(["a", true]),
        json!(["a", ["b"]]),
        json!(null),
    ] {
        assert_eq!(
            ClaimPath::from_json(&not_a_path).unwrap_err(),
            SdJwtError::InvalidClaimPath,
            "{not_a_path} was taken for a path"
        );
    }
    assert_eq!(
        ClaimPath::new(Vec::new()).unwrap_err(),
        SdJwtError::InvalidClaimPath
    );
}

// --- the RFC's own choices, as paths -------------------------------------------

/// Section 5.2 prints a presentation of the issued token of section 5.1. Its
/// Holder reveals `given_name`, `family_name`, `address` and the first of the two
/// nationalities, and the paths name those four.
#[test]
fn the_rfcs_section_5_2_choice_made_by_path_gives_the_rfcs_presentation() {
    let issued = SdJwt::parse(S51_ISSUED).expect("parses");
    let theirs = SdJwt::parse(S52_PRESENTATION).expect("parses");

    let digests = issued
        .digests_for_paths(&[
            ClaimPath::claim("given_name"),
            ClaimPath::claim("family_name"),
            ClaimPath::claim("address"),
            ClaimPath::claim("nationalities").index(0),
        ])
        .expect("in the token");

    let presented = issued
        .present(&digests.iter().map(String::as_str).collect::<Vec<_>>())
        .expect("presentable");
    assert_eq!(
        presented
            .disclosures()
            .iter()
            .map(Disclosure::digest)
            .collect::<BTreeSet<_>>(),
        digests_of(theirs.disclosures())
    );
    assert_eq!(
        Value::Object(presented.disclosed_payload().expect("processes")),
        serde_json::from_str::<Value>(S52_PROCESSED).expect("JSON")
    );
}

/// Appendix A.3 prints the issued token and the presentation made from it. The
/// presentation shows one of the age thresholds and the nationality, and the paths
/// name those two.
#[test]
fn the_rfcs_appendix_a3_choice_made_by_path_gives_the_rfcs_presentation() {
    let issued = SdJwt::parse(A3_ISSUED).expect("parses");
    let theirs = SdJwt::parse(A3_PRESENTATION).expect("parses");

    let digests = issued
        .digests_for_paths(&[
            ClaimPath::claim("age_equal_or_over").key("18"),
            ClaimPath::claim("nationalities").index(0),
        ])
        .expect("in the token");

    let presented = issued
        .present(&digests.iter().map(String::as_str).collect::<Vec<_>>())
        .expect("presentable");
    assert_eq!(presented.jwt(), theirs.jwt());
    assert_eq!(
        presented
            .disclosures()
            .iter()
            .map(Disclosure::digest)
            .collect::<BTreeSet<_>>(),
        digests_of(theirs.disclosures())
    );
    assert_eq!(
        Value::Object(presented.disclosed_payload().expect("processes")),
        serde_json::from_str::<Value>(A3_PROCESSED).expect("JSON")
    );
}

// --- a model written without the walk -------------------------------------------

/// A claim tree and which of its members are hidden.
#[derive(Debug, Clone)]
enum Node {
    Leaf(Value),
    Object(Vec<Slot>),
    Array(Vec<Slot>),
}

#[derive(Debug, Clone)]
struct Slot {
    hidden: bool,
    node: Node,
}

fn leaf() -> impl Strategy<Value = Node> {
    prop_oneof![
        any::<bool>().prop_map(|b| Node::Leaf(json!(b))),
        (0u32..100).prop_map(|n| Node::Leaf(json!(n))),
        "[a-z]{1,4}".prop_map(|s| Node::Leaf(json!(s))),
    ]
}

fn slot(node: impl Strategy<Value = Node>) -> impl Strategy<Value = Slot> {
    (any::<bool>(), node).prop_map(|(hidden, node)| Slot { hidden, node })
}

fn tree() -> impl Strategy<Value = Node> {
    leaf().prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(slot(inner.clone()), 1..4).prop_map(Node::Object),
            prop::collection::vec(slot(inner), 1..4).prop_map(Node::Array),
        ]
    })
}

/// One node, built: how it is signed, the Disclosures under it, and for every path
/// inside it the Disclosures that path needs, found here while building.
struct Built {
    signed: Value,
    disclosures: Vec<Disclosure>,
    /// Every path inside this node, relative to it, with the digests it needs. The
    /// empty path is the node itself.
    needs: Vec<(Vec<PathSegment>, Vec<String>)>,
}

fn build(node: &Node) -> Built {
    match node {
        Node::Leaf(value) => Built {
            signed: value.clone(),
            disclosures: Vec::new(),
            needs: vec![(Vec::new(), Vec::new())],
        },
        Node::Object(slots) => {
            let children: Vec<(String, &Slot, Built)> = slots
                .iter()
                .enumerate()
                .map(|(i, slot)| (format!("k{i}"), slot, build(&slot.node)))
                .collect();
            let members: Map<String, Value> = children
                .iter()
                .map(|(name, _, child)| (name.clone(), child.signed.clone()))
                .collect();
            let hidden: Vec<&str> = children
                .iter()
                .filter(|(_, slot, _)| slot.hidden)
                .map(|(name, _, _)| name.as_str())
                .collect();
            let (signed, own) = conceal(&members, |name| hidden.contains(&name)).expect("conceals");

            let mut disclosures = own.clone();
            let mut needs = vec![(Vec::new(), Vec::new())];
            for (name, slot, child) in children {
                let own_digest = slot.hidden.then(|| digest_named(&own, &name));
                for (relative, needed) in child.needs {
                    let mut path = vec![PathSegment::Key(name.clone())];
                    path.extend(relative);
                    let mut all: Vec<String> = own_digest.iter().cloned().collect();
                    all.extend(needed);
                    needs.push((path, all));
                }
                disclosures.extend(child.disclosures);
            }
            Built {
                signed: Value::Object(signed),
                disclosures,
                needs,
            }
        }
        Node::Array(slots) => {
            let children: Vec<(&Slot, Built)> =
                slots.iter().map(|slot| (slot, build(&slot.node))).collect();
            let items: Vec<Value> = children.iter().map(|(_, c)| c.signed.clone()).collect();
            let (signed, own) = conceal_elements(&items, |position, _| children[position].0.hidden);

            let mut hidden_digests = own.iter().map(Disclosure::digest);
            let mut disclosures = own.clone();
            let mut needs = vec![(Vec::new(), Vec::new())];
            for (position, (slot, child)) in children.into_iter().enumerate() {
                let own_digest = slot
                    .hidden
                    .then(|| hidden_digests.next().expect("one each"));
                for (relative, needed) in child.needs {
                    let mut path = vec![PathSegment::Index(position)];
                    path.extend(relative);
                    let mut all: Vec<String> = own_digest.iter().cloned().collect();
                    all.extend(needed);
                    needs.push((path, all));
                }
                disclosures.extend(child.disclosures);
            }
            Built {
                signed: Value::Array(signed),
                disclosures,
                needs,
            }
        }
    }
}

proptest! {
    /// For any tree, any hiding of its members, and any paths into it, the
    /// Disclosures `digests_for_paths` returns are exactly the ones the model found
    /// while building the token: every one a path needs, and nothing a path does not.
    /// The result is presentable, and revealing them leaves each named claim in place.
    #[test]
    fn the_disclosures_chosen_are_exactly_the_ones_the_paths_need(
        // The root is an object, because a payload is.
        slots in prop::collection::vec(slot(tree()), 1..4),
        picks in prop::collection::vec(any::<usize>(), 1..4),
    ) {
        let root = Node::Object(slots);
        let built = build(&root);
        let Value::Object(signed) = built.signed.clone() else { unreachable!("an object") };
        let token = SdJwt::new(
            stub_jwt(&build_payload(signed, !built.disclosures.is_empty())),
            built.disclosures.clone(),
        );

        // Every path except the empty one, which names no claim.
        let claims: Vec<&(Vec<PathSegment>, Vec<String>)> =
            built.needs.iter().filter(|(path, _)| !path.is_empty()).collect();
        let picked: Vec<&(Vec<PathSegment>, Vec<String>)> =
            picks.iter().map(|i| claims[i % claims.len()]).collect();
        let paths: Vec<ClaimPath> = picked
            .iter()
            .map(|(path, _)| ClaimPath::new(path.clone()).expect("not empty"))
            .collect();

        let expected: BTreeSet<String> =
            picked.iter().flat_map(|(_, needed)| needed.iter().cloned()).collect();
        let digests = token.digests_for_paths(&paths).expect("every path is in the token");
        prop_assert_eq!(digests.iter().cloned().collect::<BTreeSet<_>>(), expected);

        // Each Disclosure once, in the order the token holds them.
        let in_token_order: Vec<String> = built
            .disclosures
            .iter()
            .map(Disclosure::digest)
            .filter(|d| digests.contains(d))
            .collect();
        prop_assert_eq!(&digests, &in_token_order);

        let refs: Vec<&str> = digests.iter().map(String::as_str).collect();
        prop_assert!(token.present(&refs).is_ok(), "the choice is not presentable");
    }
}
