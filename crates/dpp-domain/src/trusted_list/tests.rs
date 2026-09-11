//! Trusted-list vocabulary, and the rule that qualified status has a date.

use super::*;
use chrono::{TimeZone as _, Utc};

fn at(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

/// The two statuses that carry meaning under the Regulation round-trip.
#[test]
fn the_qualified_axis_round_trips_through_its_uris() {
    assert_eq!(
        TrustServiceStatus::from_uri(GRANTED_URI),
        TrustServiceStatus::Granted
    );
    assert_eq!(
        TrustServiceStatus::from_uri(WITHDRAWN_URI),
        TrustServiceStatus::Withdrawn
    );
    assert_eq!(TrustServiceStatus::Granted.as_uri(), GRANTED_URI);
    assert_eq!(TrustServiceStatus::Withdrawn.as_uri(), WITHDRAWN_URI);
}

/// Only `granted` is granted — every other status, named or not.
///
/// The statuses below are real TS 119 612 values and each reads as though it
/// might be a weaker kind of qualified. `undersupervision` and `accredited` are
/// pre-eIDAS survivals from the Directive 1999/93/EC regime;
/// `recognisedatnationallevel` belongs to nationally-defined services. None of
/// them confers qualified status under Regulation (EU) No 910/2014, and the
/// point of asserting it is that the names do not make that obvious.
#[test]
fn no_other_status_is_a_lesser_granted() {
    for uri in [
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/undersupervision",
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/accredited",
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/recognisedatnationallevel",
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/setbynationallaw",
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/supervisionceased",
    ] {
        let status = TrustServiceStatus::from_uri(uri);
        assert!(
            !status.is_granted(),
            "{uri} is not qualified status under 910/2014"
        );
        assert_eq!(status.as_uri(), uri, "and it round-trips unchanged");
    }
}

/// A status this build has never seen is readable, and is not granted.
///
/// A Member State publishing a status URI added after this crate was built is
/// not malformed, it is newer. Refusing it would turn a routine publication into
/// an outage; treating it as granted would be far worse.
#[test]
fn an_unknown_status_is_readable_and_not_granted() {
    let future = TrustServiceStatus::from_uri(
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/somethingaddedin2030",
    );
    assert!(!future.is_granted());
    assert!(matches!(future, TrustServiceStatus::Other(_)));
}

/// The Art. 39a service type is distinguishable from its non-qualified twin.
///
/// The two URIs differ by four characters and the legal effect differs
/// completely: only the qualified one satisfies the creation-device limb of
/// Art. 3(27). Pinned because a substitution here would be invisible on reading.
#[test]
fn the_remote_seal_device_service_types_are_not_interchangeable() {
    let qualified = TrustServiceType::new(TrustServiceType::REMOTE_QSEAL_CD_MANAGEMENT);
    let plain = TrustServiceType::new(TrustServiceType::REMOTE_SEAL_CD_MANAGEMENT);

    assert_ne!(qualified, plain);
    assert!(qualified.looks_qualified());
    assert!(
        !plain.looks_qualified(),
        "managing a non-qualified creation device is not the Art. 39a service"
    );
}

/// Qualified status is asked about a moment, not about now.
///
/// The defect this whole module exists to prevent. A provider granted in 2026
/// and withdrawn in 2029 sealed a perfectly good passport in 2027. A
/// present-tense check in 2030 reports that seal as unqualified, and it is
/// wrong: Art. 32(1)(b), applied to seals by Art. 40, asks whether the
/// certificate was issued by a qualified provider **at the time of signing**.
#[test]
fn a_withdrawal_does_not_reach_back_to_seals_made_while_granted() {
    let history = TrustServiceHistory::new(vec![
        TrustServiceStatusPeriod {
            status: TrustServiceStatus::Granted,
            starting_at: at(2026, 1, 1),
        },
        TrustServiceStatusPeriod {
            status: TrustServiceStatus::Withdrawn,
            starting_at: at(2029, 6, 1),
        },
    ]);

    assert!(
        history.was_granted_at(at(2027, 5, 1)),
        "the seal was made while the provider held qualified status"
    );
    assert!(
        !history.was_granted_at(at(2030, 1, 1)),
        "and the provider does not hold it now"
    );
}

/// And a later grant does not reach back either.
///
/// The opposite error, and the more dangerous one: it would certify a seal that
/// never was qualified. Asserted separately because a `status_at` that always
/// returned the newest period would pass the test above and fail this one.
#[test]
fn a_later_grant_does_not_qualify_an_earlier_seal() {
    let history = TrustServiceHistory::new(vec![
        TrustServiceStatusPeriod {
            status: TrustServiceStatus::Withdrawn,
            starting_at: at(2026, 1, 1),
        },
        TrustServiceStatusPeriod {
            status: TrustServiceStatus::Granted,
            starting_at: at(2029, 6, 1),
        },
    ]);

    assert!(
        !history.was_granted_at(at(2027, 5, 1)),
        "the provider was not qualified when this seal was made"
    );
    assert!(history.was_granted_at(at(2030, 1, 1)));
}

/// Periods arrive in no particular order, and the answer does not depend on it.
///
/// Trusted-list history instances carry no ordering guarantee. Requiring the
/// caller to sort first is a precondition that gets forgotten, and the failure
/// would be silent and wrong rather than loud.
#[test]
fn unordered_periods_give_the_same_answer() {
    let ordered = vec![
        TrustServiceStatusPeriod {
            status: TrustServiceStatus::Granted,
            starting_at: at(2026, 1, 1),
        },
        TrustServiceStatusPeriod {
            status: TrustServiceStatus::Withdrawn,
            starting_at: at(2029, 6, 1),
        },
    ];
    let mut shuffled = ordered.clone();
    shuffled.reverse();

    for when in [at(2027, 5, 1), at(2030, 1, 1)] {
        assert_eq!(
            TrustServiceHistory::new(ordered.clone()).was_granted_at(when),
            TrustServiceHistory::new(shuffled.clone()).was_granted_at(when),
        );
    }
}

/// "Not known" is not "not granted", and the API keeps them apart.
///
/// A list whose history begins after the moment asked about says nothing about
/// that moment. `was_granted_at` answers `false`, which is the safe reading, but
/// a compliance finding needs to distinguish a provider that was refused from
/// one the list simply does not cover — so `status_at` returns `None` rather
/// than synthesising a withdrawal.
#[test]
fn a_moment_before_the_history_is_unknown_not_withdrawn() {
    let history = TrustServiceHistory::new(vec![TrustServiceStatusPeriod {
        status: TrustServiceStatus::Granted,
        starting_at: at(2026, 1, 1),
    }]);

    assert_eq!(history.status_at(at(2020, 1, 1)), None);
    assert!(!history.was_granted_at(at(2020, 1, 1)));
    assert_eq!(
        history.status_at(at(2026, 1, 1)),
        Some(&TrustServiceStatus::Granted),
        "the boundary is inclusive: a status applies from the instant it starts"
    );
}

/// An empty history decides nothing.
#[test]
fn an_empty_history_is_unknown() {
    let history = TrustServiceHistory::new(vec![]);
    assert_eq!(history.status_at(at(2027, 1, 1)), None);
    assert!(!history.was_granted_at(at(2027, 1, 1)));
}
