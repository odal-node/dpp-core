//! Every `PassportStatus` variant is reachable and enumerated.

use super::status::PassportStatus;

/// `ALL` must list every variant.
///
/// The match is exhaustive with no catch-all, so a new variant stops this
/// compiling; the length assertion then fails until `ALL` is updated. Both
/// stages matter — downstream consumers cannot enumerate this enum at all
/// (it is `#[non_exhaustive]`) and inherit any gap in `ALL` silently. The
/// engine's API description omitted `superseded` and `deactivated` for
/// exactly that reason.
#[test]
fn all_lists_every_variant() {
    for status in PassportStatus::ALL {
        match status {
            PassportStatus::Draft
            | PassportStatus::Published
            | PassportStatus::Suspended
            | PassportStatus::Archived
            | PassportStatus::Superseded
            | PassportStatus::Deactivated => {}
        }
    }
    assert_eq!(
        PassportStatus::ALL.len(),
        6,
        "a variant was added to the match above but not to ALL"
    );
}

/// Every listed status must have a distinct wire string, since `ALL` is what
/// a consumer builds its documented value set from.
#[test]
fn all_wire_strings_are_distinct() {
    let mut seen = std::collections::BTreeSet::new();
    for status in PassportStatus::ALL {
        assert!(
            seen.insert(status.to_string()),
            "two statuses serialise to the same wire string: {status}"
        );
    }
}
