//! The obligation calendar with a signed bundle's schedule laid over it: what a
//! bundle may add, what it may never move, and which bundles are not read.
//!
//! Driven through the real `verify_bundle` with a verifier that accepts every
//! signature, so the acceptance each test reads is a genuinely verified one
//! without a signing key in this crate's tests.

use std::collections::BTreeMap;

use base64::Engine;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use dpp_rules::bundle::{
    AcceptancePolicy, JwsVerify, RulesetAcceptance, RulesetError, RulesetManifest, SignedBundle,
    content_hash, verify_bundle,
};
use serde_json::{Value, json};

use super::*;
use crate::instrument::{DateBasis, InstrumentCatalog, ObligationDate, PassportObligation};

struct AcceptsEverySignature;
impl JwsVerify for AcceptsEverySignature {
    fn verify_eddsa(&self, _jws: &str, _public_key_b64: &str) -> Result<bool, RulesetError> {
        Ok(true)
    }
}

const VERSION: &str = "2027-Q1.1";

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

fn manifest(content: &Value) -> RulesetManifest {
    RulesetManifest {
        bundle_version: VERSION.to_owned(),
        effective_date: at(2027, 1, 1),
        act_citations: Vec::new(),
        schema_versions: BTreeMap::new(),
        content_sha256: content_hash(content).unwrap(),
    }
}

/// `content` as a bundle that passed `verify_bundle`.
fn verified(content: Value) -> RulesetAcceptance {
    let manifest = manifest(&content);
    let payload = serde_json::to_vec(&manifest).unwrap();
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
    let bundle = SignedBundle {
        manifest_jws: format!("aGVhZGVy.{payload_b64}.c2ln"),
        content,
    };
    let policy = AcceptancePolicy {
        now: at(2027, 1, 2),
        in_force: None,
    };
    verify_bundle(
        &bundle,
        "pinned-publisher-key",
        &AcceptsEverySignature,
        &policy,
    )
    .unwrap()
}

/// A schedule entry as a bundle carries it.
fn entry(id: &str, date: &str, basis: &str) -> Value {
    json!({
        "id": id,
        "date": date,
        "kind": "passportStart",
        "citation": "A later act, Art. 1",
        "summary": "A passport is required from this date.",
        "productGroup": "textile",
        "basis": basis,
    })
}

/// A verified bundle carrying `entries` as its schedule.
fn schedule(entries: Vec<Value>) -> RulesetAcceptance {
    verified(json!({ "obligations": entries }))
}

fn with_schedule(entries: Vec<Value>) -> Result<ObligationCalendar, ScheduleBundleError> {
    ObligationCalendar::with_bundle(&InstrumentCatalog::new(), &schedule(entries))
}

/// The origin an event or conflict from `acceptance` must record.
fn origin(acceptance: &RulesetAcceptance) -> BundleOrigin {
    BundleOrigin {
        bundle_version: VERSION.to_owned(),
        content_sha256: acceptance.manifest().content_sha256.clone(),
    }
}

fn find<'a>(calendar: &'a ObligationCalendar, id: &str) -> &'a ObligationEvent {
    calendar.events().iter().find(|e| e.id == id).unwrap()
}

#[test]
fn a_bundle_adds_a_date_this_build_does_not_hold() {
    let calendar = with_schedule(vec![entry(
        "a-later-act.passport.textile",
        "2030-07-01",
        "sourced",
    )])
    .unwrap();

    let event = find(&calendar, "a-later-act.passport.textile");
    assert_eq!(event.date, day(2030, 7, 1));
    assert_eq!(event.basis, DateBasis::Sourced);
    assert_eq!(event.product_group.as_deref(), Some("textile"));
    assert_eq!(
        event.bundle.as_ref().unwrap().bundle_version,
        VERSION,
        "an announced date says which bundle it came from"
    );
    assert!(calendar.conflicts().is_empty());

    let window: Vec<&str> = calendar
        .between(day(2030, 7, 1), day(2030, 7, 1))
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(window, ["a-later-act.passport.textile"]);
    assert!(
        !ObligationCalendar::new()
            .events()
            .iter()
            .any(|e| e.id == event.id),
        "the build itself does not hold the date"
    );
    assert!(
        calendar
            .events()
            .windows(2)
            .all(|w| (w[0].date, &w[0].id) <= (w[1].date, &w[1].id)),
        "a bundled event is sorted into place"
    );
}

/// A rule's date is the one the publish gate applies. A bundle moving it earlier
/// is recorded and not announced.
#[test]
fn a_bundle_cannot_move_a_rule_date_earlier() {
    let acceptance = schedule(vec![entry(
        "battery.due-diligence-report",
        "2027-02-18",
        "sourced",
    )]);
    let calendar = ObligationCalendar::with_bundle(&InstrumentCatalog::new(), &acceptance).unwrap();

    assert_eq!(calendar.events(), ObligationCalendar::new().events());
    assert_eq!(
        calendar.conflicts(),
        [ScheduleConflict {
            id: "battery.due-diligence-report".to_owned(),
            build_date: day(2027, 8, 18),
            bundle_date: day(2027, 2, 18),
            bundle: origin(&acceptance),
        }]
    );
}

/// The same for an act's sourced passport start date, moved later.
#[test]
fn a_bundle_cannot_move_a_sourced_passport_start_later() {
    let calendar = with_schedule(vec![entry(
        "battery-reg-2023-1542.passport.battery",
        "2028-02-18",
        "sourced",
    )])
    .unwrap();

    let event = find(&calendar, "battery-reg-2023-1542.passport.battery");
    assert_eq!(event.date, day(2027, 2, 18));
    assert!(event.bundle.is_none());
    assert_eq!(calendar.conflicts().len(), 1);
    assert_eq!(calendar.conflicts()[0].build_date, day(2027, 2, 18));
    assert_eq!(calendar.conflicts()[0].bundle_date, day(2028, 2, 18));
}

#[test]
fn a_bundle_agreeing_with_a_sourced_date_changes_nothing() {
    let calendar = with_schedule(vec![entry(
        "battery.due-diligence-report",
        "2027-08-18",
        "sourced",
    )])
    .unwrap();

    assert_eq!(calendar, ObligationCalendar::new());
}

/// A bundle fills an assumption, in either direction. The earlier date is the
/// one a node most needs to hear about.
#[test]
fn a_bundle_fills_an_assumed_date_even_earlier() {
    let mut catalog = InstrumentCatalog::new();
    let mut assumed = catalog.get("toy-safety-2025-2509").unwrap().clone();
    assumed.id = "an-assumed-act".to_owned();
    assumed.passport = PassportObligation::Required {
        from: Some(ObligationDate {
            date: "2031-01-01".to_owned(),
            basis: DateBasis::Assumed,
        }),
    };
    for binding in &mut assumed.product_groups {
        binding.passport = None;
    }
    catalog.register(assumed).unwrap();

    let acceptance = verified(json!({
        "obligations": [entry("an-assumed-act.passport.toy", "2030-06-01", "sourced")],
    }));
    let calendar = ObligationCalendar::with_bundle(&catalog, &acceptance).unwrap();

    let event = find(&calendar, "an-assumed-act.passport.toy");
    assert_eq!(event.date, day(2030, 6, 1));
    assert_eq!(event.basis, DateBasis::Sourced);
    assert!(event.bundle.is_some());
    assert!(calendar.conflicts().is_empty());
    assert_eq!(
        calendar
            .events()
            .iter()
            .filter(|e| e.id == "an-assumed-act.passport.toy")
            .count(),
        1,
        "the bundle's entry replaces the assumption rather than joining it"
    );
}

/// A node's compiled-in baseline has no signature to check, so its content is
/// never read as a schedule.
#[test]
fn an_unverified_bundle_is_refused() {
    let content = json!({
        "obligations": [entry("a-later-act.passport.textile", "2030-07-01", "sourced")],
    });
    let baseline = RulesetAcceptance::unverified_baseline(manifest(&content), content);

    let refused = ObligationCalendar::with_bundle(&InstrumentCatalog::new(), &baseline);
    assert_eq!(
        refused,
        Err(ScheduleBundleError::Unverified {
            bundle_version: VERSION.to_owned()
        })
    );
}

/// A node whose bundle carries other content only has the calendar it would
/// have had without one.
#[test]
fn a_bundle_without_a_schedule_gives_the_built_in_calendar() {
    let acceptance = verified(json!({ "rulesets": {} }));
    let calendar = ObligationCalendar::with_bundle(&InstrumentCatalog::new(), &acceptance);
    assert_eq!(calendar, Ok(ObligationCalendar::new()));
}

#[test]
fn a_malformed_schedule_is_refused_whole() {
    let mut claims_its_origin = entry("a-later-act.passport.textile", "2030-07-01", "sourced");
    claims_its_origin["bundle"] = json!({ "bundleVersion": "x", "contentSha256": "y" });
    let mut no_basis = entry("a-later-act.passport.textile", "2030-07-01", "sourced");
    no_basis.as_object_mut().unwrap().remove("basis");

    let cases: [(&str, Value); 5] = [
        ("not a list", json!({ "obligations": {} })),
        (
            "an entry naming its own origin",
            json!({ "obligations": [claims_its_origin] }),
        ),
        (
            "an entry with no basis",
            json!({ "obligations": [no_basis] }),
        ),
        (
            "an impossible date",
            json!({ "obligations": [entry("a-later-act.passport.textile", "2030-02-30", "sourced")] }),
        ),
        (
            "a repeated id",
            json!({ "obligations": [
                entry("a-later-act.passport.textile", "2030-07-01", "sourced"),
                entry("a-later-act.passport.textile", "2031-07-01", "sourced"),
            ] }),
        ),
    ];
    for (case, content) in cases {
        let refused =
            ObligationCalendar::with_bundle(&InstrumentCatalog::new(), &verified(content));
        assert!(
            matches!(refused, Err(ScheduleBundleError::Malformed { .. })),
            "{case}: {refused:?}"
        );
    }
}

#[test]
fn a_bundled_event_serialises_with_its_origin() {
    let acceptance = schedule(vec![entry(
        "a-later-act.passport.textile",
        "2030-07-01",
        "assumed",
    )]);
    let calendar = ObligationCalendar::with_bundle(&InstrumentCatalog::new(), &acceptance).unwrap();
    let wire = serde_json::to_value(find(&calendar, "a-later-act.passport.textile")).unwrap();
    assert_eq!(wire["basis"], "assumed");
    assert_eq!(
        wire["bundle"],
        serde_json::to_value(origin(&acceptance)).unwrap()
    );
    assert_eq!(wire["bundle"]["bundleVersion"], VERSION);
    assert_eq!(
        wire["bundle"]["contentSha256"],
        acceptance.manifest().content_sha256.as_str()
    );
}
