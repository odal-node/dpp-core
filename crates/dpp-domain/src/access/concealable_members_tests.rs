//! `concealable_members`: how many non-public members a policy names at one position.
//!
//! The count sizes an issuer's `_sd` array, so a wrong answer is a leak (too low) or a
//! waste (too high). These pin the position rule, the two scopes, and what `0` means.

use super::{DocumentScope, ProductGroupAccessPolicy};
use crate::disclosure::{Disclosure, PASSPORT_FIELD_DISCLOSURE};

fn policy(keys: &[(&str, Disclosure)]) -> ProductGroupAccessPolicy {
    let mut policy = ProductGroupAccessPolicy::passport_default();
    policy.field_disclosure = keys
        .iter()
        .map(|(key, class)| ((*key).to_owned(), *class))
        .collect();
    policy
}

fn at(policy: &ProductGroupAccessPolicy, path: &[&str]) -> usize {
    policy.concealable_members(path, DocumentScope::ProductGroupData)
}

#[test]
fn a_member_is_counted_at_the_position_its_key_is_written_for() {
    let policy = policy(&[
        ("a", Disclosure::Restricted),
        ("hid", Disclosure::Individual),
        ("pub.b", Disclosure::Restricted),
        ("pub.c", Disclosure::Conformity),
        ("hid.e", Disclosure::Individual),
        ("deeper.x.y", Disclosure::Restricted),
    ]);

    assert_eq!(at(&policy, &[]), 2, "a and hid, the bare names");
    assert_eq!(at(&policy, &["pub"]), 2);
    assert_eq!(at(&policy, &["hid"]), 1);
    assert_eq!(at(&policy, &["deeper", "x"]), 1);
}

/// Matching is by suffix, so a bare name classes a member at any depth. Counting is by
/// position: if a bare name counted at every nested object, each would be padded with the
/// names of all the others.
#[test]
fn a_bare_name_is_not_counted_below_the_root_and_a_dotted_key_not_by_suffix() {
    let policy = policy(&[
        ("a", Disclosure::Restricted),
        ("pub.b", Disclosure::Restricted),
        ("deeper.x.y", Disclosure::Restricted),
    ]);

    assert_eq!(at(&policy, &["pub"]), 1, "b only, not a");
    assert_eq!(at(&policy, &["x"]), 0, "deeper.x.y is not a key of x");
    assert_eq!(at(&policy, &["deeper"]), 0);
    assert_eq!(
        at(&policy, &["other", "pub"]),
        0,
        "the whole path must match"
    );
}

#[test]
fn a_path_is_matched_the_way_the_policy_matches_names() {
    let policy = policy(&[
        ("usageHistory.recordedAt", Disclosure::Individual),
        ("usage_history.recordedAt", Disclosure::Individual),
        ("usageHistory.note", Disclosure::Public),
    ]);

    assert_eq!(
        at(&policy, &["usage_history"]),
        1,
        "one member written through two spellings of its parent, and a public one is not counted"
    );
}

/// A policy lookup reads `recordedAt` and `recorded_at` as one name, but a document holds
/// them as two members and the issuer hides both. One count for the two would leave the
/// object that holds both a digest longer than the object that holds neither.
#[test]
fn two_spellings_of_a_name_are_two_members() {
    let policy = policy(&[
        ("usageHistory.recordedAt", Disclosure::Individual),
        ("usageHistory.recorded_at", Disclosure::Individual),
    ]);

    assert_eq!(at(&policy, &["usageHistory"]), 2);
}

/// The issuer hides a member by what `disclosure_for_path` says, and that raises a
/// universal conformity field above the class a schema gave it, at any depth.
#[test]
fn a_member_the_conformity_floor_raises_is_counted() {
    let policy = policy(&[
        ("container.secret", Disclosure::Restricted),
        ("container.jwsSignature", Disclosure::Public),
        ("container.note", Disclosure::Public),
        ("jwsSignature", Disclosure::Public),
    ]);

    assert_eq!(
        policy.disclosure_for_path(
            &["container", "jwsSignature"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Conformity,
        "the premise: the issuer hides it"
    );
    assert_eq!(at(&policy, &["container"]), 2, "secret and jwsSignature");
    assert_eq!(at(&policy, &[]), 1, "the bare jwsSignature, raised as well");
}

#[test]
fn a_policy_that_hides_what_it_does_not_list_has_no_count() {
    let mut policy = policy(&[("a", Disclosure::Restricted)]);
    assert_eq!(at(&policy, &[]), 1);
    policy.default_disclosure = Disclosure::Restricted;
    assert_eq!(at(&policy, &[]), 0);
}

#[test]
fn the_envelope_root_counts_the_envelope_fields_a_passport_can_carry() {
    let mut policy = ProductGroupAccessPolicy::passport_default();
    let named = |scope| policy.concealable_members(&[], scope);

    let mut expected: Vec<String> = PASSPORT_FIELD_DISCLOSURE
        .iter()
        .filter(|(_, class)| *class != Disclosure::Public)
        .map(|(name, _)| (*name).to_ascii_lowercase())
        .collect();
    for universal in [
        "jwsSignature",
        "complianceReport",
        "auditHistory",
        "supplyChainTrace",
    ] {
        let name = universal.to_ascii_lowercase();
        if !expected.contains(&name) {
            expected.push(name);
        }
    }
    assert_eq!(named(DocumentScope::Envelope), expected.len());
    assert_eq!(
        named(DocumentScope::ProductGroupData),
        0,
        "the envelope's names are not members of productGroupData"
    );

    policy.public_envelope_fields.insert("batchId".to_owned());
    assert_eq!(
        policy.concealable_members(&[], DocumentScope::Envelope),
        expected.len() - 1,
        "a field the product group opens is not one the issuer can conceal"
    );
}

#[test]
fn an_envelope_object_below_the_root_has_no_count() {
    let policy = ProductGroupAccessPolicy::passport_default();
    assert_eq!(
        policy.concealable_members(&["operator"], DocumentScope::Envelope),
        0
    );
}
