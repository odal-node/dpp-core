//! A printed data carrier resolves to the passport it was printed for.
//!
//! # Why this is one test across three crates
//!
//! The carrier is built in `dpp-digital-link`, read back by the same crate's
//! parser, and resolved through `dpp-domain`'s repository port. Each half was
//! tested on its own and each half passed while the whole did not: the carrier
//! printed a serial derived from the passport id, the lookup compared the
//! manufacturer's serial, and every label resolved to nothing. Nothing short of
//! building a carrier and resolving it could see that, so that is what this does.

use std::sync::Mutex;

use async_trait::async_trait;
use dpp_digital_link::{DigitalLink, DigitalLinkError, ai_spec, build_qr_url};
use dpp_domain::ports::passport_repo::PassportRepository;
use dpp_domain::{
    BatteryData, DppError, Passport, PassportId, PassportStatus, ProductGroup, ProductGroupData,
    ProductIdentifier,
};
use dpp_rules::common::identifier::MAX_GS1_SERIAL_CHARS;
use dpp_tests::fixtures::base_passport;

const RESOLVER: &str = "https://id.example.com";
const GTIN: &str = "09506000134352";

/// Only what the lookups under test read: `create` and `list`. The rest of the
/// port is never reached, and says so if it is.
#[derive(Default)]
struct Store(Mutex<Vec<Passport>>);

#[async_trait]
impl PassportRepository for Store {
    async fn create(&self, passport: Passport) -> Result<Passport, DppError> {
        self.0.lock().expect("unpoisoned").push(passport.clone());
        Ok(passport)
    }
    async fn list(
        &self,
        _status: Option<PassportStatus>,
        _q: Option<&str>,
        _facility_id: Option<&str>,
        _limit: u32,
        _offset: u32,
    ) -> Result<Vec<Passport>, DppError> {
        Ok(self.0.lock().expect("unpoisoned").clone())
    }
    async fn find_by_id(&self, _id: PassportId) -> Result<Option<Passport>, DppError> {
        unreachable!("not used by carrier resolution")
    }
    async fn find_published_by_id(&self, _id: PassportId) -> Result<Option<Passport>, DppError> {
        unreachable!("not used by carrier resolution")
    }
    async fn find_by_id_any_status(&self, _id: PassportId) -> Result<Option<Passport>, DppError> {
        unreachable!("not used by carrier resolution")
    }
    async fn update(&self, _passport: Passport) -> Result<Passport, DppError> {
        unreachable!("not used by carrier resolution")
    }
    async fn update_status(
        &self,
        _id: PassportId,
        _status: PassportStatus,
    ) -> Result<Passport, DppError> {
        unreachable!("not used by carrier resolution")
    }
    async fn count(
        &self,
        _status: Option<PassportStatus>,
        _facility_id: Option<&str>,
    ) -> Result<u64, DppError> {
        unreachable!("not used by carrier resolution")
    }
}

fn battery(identifier: serde_json::Value) -> Passport {
    let data: BatteryData = serde_json::from_value(serde_json::json!({
        "productIdentifier": identifier,
        "batteryChemistry": "LFP",
        "nominalVoltageV": 3.2,
        "nominalCapacityAh": 100.0,
        "co2ePerUnitKg": 12.0,
        "batteryType": "ev",
    }))
    .expect("a minimal battery record");
    let mut passport = base_passport(
        ProductGroup::Battery,
        ProductGroupData::Battery(Box::new(data)),
        "2.7.0",
    );
    passport.batch_id = None;
    passport
}

fn gs1_battery() -> Passport {
    battery(serde_json::json!({ "scheme": "gs1", "gtin": GTIN }))
}

fn carrier(passport: &Passport) -> String {
    build_qr_url(RESOLVER, passport)
        .expect("a valid carrier")
        .expect("a GS1-identified passport has a GS1 carrier")
}

/// Model, lot and unit passports under one GTIN, plus one whose operator
/// attributed its own serial: each carrier resolves to its passport and no
/// other.
#[tokio::test]
async fn a_printed_carrier_resolves_to_the_passport_it_was_printed_for() {
    let store = Store::default();

    let model = gs1_battery();

    let mut lot = gs1_battery();
    lot.batch_id = Some("LOT-A".into());

    let mut unit = gs1_battery();
    unit.batch_id = Some("LOT-A".into());
    unit.serial_number = Some("SN-0001".into());

    let mut attributed = gs1_battery();
    attributed.serial_number = Some("SN-0002".into());
    attributed.carrier_serial = Some("SN-0002".into());

    for passport in [&model, &lot, &unit, &attributed] {
        store.create(passport.clone()).await.expect("stored");
    }

    for passport in [&model, &lot, &unit, &attributed] {
        let label = carrier(passport);
        let link = DigitalLink::parse(&label).expect("our own carrier parses");
        let identifier = ProductIdentifier::gs1(link.gtin().expect("keyed on a GTIN").clone());
        let serial = link.serial().expect("the carrier prints AI 21");

        let found = store
            .find_by_carrier_serial(&identifier, serial)
            .await
            .expect("lookup");
        let ids: Vec<_> = found.iter().map(|p| p.id).collect();
        assert_eq!(ids, [passport.id], "{label}");
    }
}

/// The manufacturer's serial is a fact about the unit, not a choice of what to
/// print. Recording it leaves the carrier alone; attributing a carrier serial
/// is what moves it.
#[test]
fn an_item_serial_does_not_change_the_carrier() {
    let mut passport = gs1_battery();
    let before = carrier(&passport);

    passport.serial_number = Some("SN-2026-00042".into());
    assert_eq!(carrier(&passport), before);

    passport.carrier_serial = Some("SN-2026-00042".into());
    assert_eq!(
        carrier(&passport),
        format!("{RESOLVER}/01/{GTIN}/21/SN-2026-00042")
    );
}

/// The lot is operator free text and adds nothing to resolution, so it is not
/// printed however the record is batched.
#[test]
fn the_carrier_prints_no_lot() {
    let mut passport = gs1_battery();
    passport.batch_id = Some("LOT-A".into());
    let label = carrier(&passport);
    let link = DigitalLink::parse(&label).expect("parses");
    assert_eq!(link.batch(), None, "{label}");
    assert!(!label.contains("LOT-A"), "{label}");
}

/// A Digital Link keyed on AI 01 needs a GTIN; a passport identified under
/// EN 18219 scheme 2 or 3 has none and carries its own URL or DID instead.
#[test]
fn a_passport_without_a_gtin_has_no_gs1_carrier() {
    let passport =
        battery(serde_json::json!({ "scheme": "did", "did": "did:web:example.com:b:1" }));
    assert_eq!(build_qr_url(RESOLVER, &passport).expect("no error"), None);
}

/// An attributed serial GS1 would refuse is refused before it is printed,
/// whether or not the passport went through `validate` first.
#[test]
fn an_attributed_serial_gs1_would_reject_is_not_printed() {
    let with = |serial: &str| {
        let mut passport = gs1_battery();
        passport.carrier_serial = Some(serial.to_owned());
        build_qr_url(RESOLVER, &passport)
    };
    assert!(matches!(with(""), Err(DigitalLinkError::EmptyValue(code)) if code == "21"));
    assert!(matches!(
        with("SN 1"),
        Err(DigitalLinkError::OutsideCset82 { code, character: ' ' }) if code == "21"
    ));
    assert!(matches!(
        with("123456789012345678901"),
        Err(DigitalLinkError::ValueTooLong { code, max_len: 20, actual: 21 }) if code == "21"
    ));
}

/// The domain restates GS1's AI 21 rule because it cannot read the dictionary
/// this crate vendors. Held against that dictionary here, where both are in
/// reach, so the restatement cannot drift from its source.
#[test]
fn the_restated_ai_21_rule_matches_the_dictionary() {
    let serial = ai_spec("21").expect("AI 21 in the dictionary");
    assert_eq!(serial.max_len, MAX_GS1_SERIAL_CHARS);
    assert_eq!(
        serial.min_len, 1,
        "`check_gs1_serial` refuses an empty value"
    );
    assert!(serial.cset_82, "`check_gs1_serial` applies CSET 82");
}
