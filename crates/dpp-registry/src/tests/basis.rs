//! The basis each wire constant and status declares.

use super::super::*;

/// The submission limits, recorded so a batch implementation inherits them
/// rather than rediscovering them at the registry.
#[test]
fn the_observed_submission_limits_are_recorded() {
    assert_eq!(MAX_PASSPORTS_PER_SUBMISSION, 100);
    assert_eq!(MAX_SUBMISSION_BYTES, 1_073_741_824);
}

// ── Evidence tier ────────────────────────────────────────────────────────────

/// 🚨 Every wire constant this crate declares must state what backs it.
///
/// The table is only a contract if nothing can be added beside it. A path
/// declared in `endpoint` and left out of `ENDPOINT_BASIS` is a route with no
/// provenance — which is what every route was before the table existed, and the
/// state a reader cannot distinguish from "we checked and it is fine".
///
/// Listed here by name rather than derived, deliberately: deriving the list from
/// the table would make the table agree with itself.
#[test]
fn every_wire_constant_declares_a_basis() {
    use crate::endpoint::{
        IDEMPOTENCY_KEY_HEADER, PRODUCT_GROUPS_PATH, REGISTRATION_PATH, STATUS_PATH_TEMPLATE,
        TASK_RESULT_PATH_TEMPLATE, TASKS_PATH, TASKS_SORT_NEWEST_FIRST, TOKEN_PATH,
        TRANSFER_PATH_TEMPLATE, basis_of,
    };

    for declared in [
        REGISTRATION_PATH,
        PRODUCT_GROUPS_PATH,
        TASKS_PATH,
        TASKS_SORT_NEWEST_FIRST,
        TASK_RESULT_PATH_TEMPLATE,
        STATUS_PATH_TEMPLATE,
        TRANSFER_PATH_TEMPLATE,
        TOKEN_PATH,
        IDEMPOTENCY_KEY_HEADER,
    ] {
        assert!(
            basis_of(declared).is_some(),
            "{declared} is declared by this crate and states no basis — add it to ENDPOINT_BASIS"
        );
    }

    // And nothing in the table that the crate does not declare, so a renamed
    // constant cannot leave a stale entry behind vouching for a dead route.
    assert_eq!(
        ENDPOINT_BASIS.len(),
        9,
        "the table grew or shrank without this test being told"
    );
}

/// What is observed is observed, and what is not says so.
///
/// Pinned per constant rather than counted. A count would still pass if an
/// invented route were quietly relabelled as observed, which is the single
/// change this whole tier exists to make impossible to do silently.
#[test]
fn each_route_carries_the_tier_its_prose_claims() {
    use crate::endpoint::{
        IDEMPOTENCY_KEY_HEADER, PRODUCT_GROUPS_PATH, REGISTRATION_PATH, STATUS_PATH_TEMPLATE,
        TASK_RESULT_PATH_TEMPLATE, TASKS_PATH, TASKS_SORT_NEWEST_FIRST, TOKEN_PATH,
        TRANSFER_PATH_TEMPLATE, basis_of,
    };

    // Read off the registry's own web client, on the date it was last read.
    for observed in [
        REGISTRATION_PATH,
        PRODUCT_GROUPS_PATH,
        TASKS_PATH,
        TASKS_SORT_NEWEST_FIRST,
        TASK_RESULT_PATH_TEMPLATE,
        IDEMPOTENCY_KEY_HEADER,
    ] {
        let basis = basis_of(observed).expect("declared");
        assert!(basis.is_observed(), "{observed} is observed");
        assert_eq!(
            basis.observed_on(),
            Some("2026-10-20"),
            "an observation must carry the date it was made"
        );
    }

    // Nothing behind them. A registered record needs *a* status route,
    // transfers need *a* route, a token has to come from somewhere, and naming
    // one is not knowing it.
    for assumed in [STATUS_PATH_TEMPLATE, TRANSFER_PATH_TEMPLATE, TOKEN_PATH] {
        let basis = basis_of(assumed).expect("declared");
        assert!(!basis.is_observed(), "{assumed} rests on nothing");
        assert_eq!(basis.observed_on(), None);
    }
}

/// A value this crate never declared answers `None`, not `Assumed`.
///
/// The difference matters to the caller: `Assumed` says "ours, and unbacked",
/// which is a statement about a route we named. `None` says "we never said
/// this", which for a caller that believes it copied the route from here means
/// a typo or a stale pin — and answering `Assumed` would dress that up as an
/// expected outcome.
#[test]
fn an_undeclared_route_has_no_basis_rather_than_a_default_one() {
    assert_eq!(endpoint::basis_of("/registrations"), None);
    assert_eq!(endpoint::basis_of(""), None);
}

/// The dashboard sorts by `<field>,<direction>`, and the one value the client
/// sends by default is newest first. A caller builds its query from this value
/// and reads only the front of a long list on the strength of it, so what it
/// names is pinned: the field a task carries its submission time under, and the
/// descending direction.
#[test]
fn the_newest_first_sort_is_a_field_and_a_direction() {
    let (field, direction) = endpoint::TASKS_SORT_NEWEST_FIRST
        .split_once(',')
        .expect("a sort is a field, a comma and a direction");
    assert_eq!(field, "submittedAt");
    assert_eq!(direction, "desc");
}

/// Every error kind decides its status here, or says it has none — and every
/// one of those decisions is ours.
///
/// The exhaustive `match` inside `http_status` is what makes a new kind a
/// compile error rather than a silent default; this is the other half, asserting
/// that none of the decisions has quietly been dressed up as observed. The only
/// status anyone has seen is the replayed-key 409.
#[test]
fn every_assumed_status_says_it_is_assumed() {
    use EuRegistryErrorKind as Kind;

    let with_status = [
        (Kind::Unauthorized, 401),
        (Kind::RegistrationRejected, 422),
        (Kind::RateLimited, 429),
        (Kind::NotFound, 404),
        (Kind::RegistryInternalError, 500),
    ];
    for (kind, expected) in &with_status {
        let (status, basis) = kind.http_status().expect("this kind is a response");
        assert_eq!(status, *expected, "{kind:?}");
        assert!(
            !basis.is_observed(),
            "{kind:?} claims an observed status; only the replayed-key 409 is observed"
        );
    }

    // No response, so no status to report. Inventing one would put a number on
    // the wire that no registry chose.
    for kind in &[Kind::ConnectionFailed, Kind::Timeout, Kind::InvalidResponse] {
        assert_eq!(kind.http_status(), None, "{kind:?} describes no response");
    }
}

/// The one status with something behind it.
#[test]
fn the_replayed_key_status_is_the_only_observed_one() {
    assert_eq!(STATUS_IDEMPOTENCY_KEY_REUSED, 409);
    assert!(STATUS_IDEMPOTENCY_KEY_REUSED_BASIS.is_observed());
    assert_eq!(
        STATUS_IDEMPOTENCY_KEY_REUSED_BASIS.observed_on(),
        Some("2026-10-20")
    );
}

// ── Annex III(l): the service provider reference ─────────────────────────────
