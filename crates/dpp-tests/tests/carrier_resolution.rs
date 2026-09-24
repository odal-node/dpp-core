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
    BatteryData, DppError, Granularity, Passport, PassportId, PassportStatus, ProductGroup,
    ProductGroupData, ProductIdentifier,
};
use dpp_rules::common::identifier::{MAX_GS1_LOT_CHARS, MAX_GS1_SERIAL_CHARS};
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

fn at(level: Option<Granularity>) -> Passport {
    let mut passport = gs1_battery();
    passport.granularity = level;
    passport
}

/// Resolve a label the way a resolver does: parse it, read the qualifier the
/// carrier printed, and look that up under the label's GTIN.
async fn resolve(store: &Store, label: &str) -> Vec<PassportId> {
    let link = DigitalLink::parse(label).expect("our own carrier parses");
    let identifier = ProductIdentifier::gs1(link.gtin().expect("keyed on a GTIN").clone());
    let qualifier = link
        .carrier_qualifier()
        .expect("our own carrier names a qualifier");
    store
        .find_by_carrier(&identifier, &qualifier)
        .await
        .expect("lookup")
        .iter()
        .map(|p| p.id)
        .collect()
}

/// Passports at every level under one GTIN — model, two lots, units with and
/// without a stated level, and one whose operator attributed its own serial:
/// each carrier resolves to its passport and no other.
#[tokio::test]
async fn a_printed_carrier_resolves_to_the_passport_it_was_printed_for() {
    let store = Store::default();

    let model = at(Some(Granularity::Model));

    let mut lot_a = at(Some(Granularity::Batch));
    lot_a.batch_id = Some("LOT-A".into());

    let mut lot_b = at(Some(Granularity::Batch));
    lot_b.batch_id = Some("LOT-B".into());

    let mut unit = at(Some(Granularity::Item));
    unit.batch_id = Some("LOT-A".into());
    unit.serial_number = Some("SN-0001".into());

    let mut unstated = at(None);
    unstated.batch_id = Some("LOT-A".into());

    let mut attributed = at(None);
    attributed.serial_number = Some("SN-0002".into());
    attributed.carrier_serial = Some("SN-0002".into());

    let all = [&model, &lot_a, &lot_b, &unit, &unstated, &attributed];
    for passport in all {
        passport.validate().expect("a coherent record");
        store.create(passport.clone()).await.expect("stored");
    }

    for passport in all {
        let label = carrier(passport);
        assert_eq!(resolve(&store, &label).await, [passport.id], "{label}");
    }
}

/// What each level prints: the GTIN alone for a model, the lot for a batch,
/// and the carrier serial for a unit or a passport that states no level.
#[test]
fn the_carrier_asserts_the_level_the_passport_describes() {
    let model = at(Some(Granularity::Model));
    assert_eq!(carrier(&model), format!("{RESOLVER}/01/{GTIN}"));

    let mut lot = at(Some(Granularity::Batch));
    lot.batch_id = Some("LOT-2026/A".into());
    assert_eq!(
        carrier(&lot),
        format!("{RESOLVER}/01/{GTIN}/10/LOT-2026%2FA")
    );

    for level in [Some(Granularity::Item), None] {
        let passport = at(level);
        assert_eq!(
            carrier(&passport),
            format!(
                "{RESOLVER}/01/{GTIN}/21/{}",
                passport.id.default_carrier_serial()
            ),
            "{level:?}"
        );
    }
}

/// A label printed when every carrier carried a serial, and a label printed
/// by an earlier version with a lot before the serial, both still resolve —
/// here to a passport that now states model level.
#[tokio::test]
async fn a_serial_label_printed_before_the_level_was_read_still_resolves() {
    let store = Store::default();
    let model = at(Some(Granularity::Model));
    store.create(model.clone()).await.expect("stored");

    let serial = model.effective_carrier_serial();
    for label in [
        format!("{RESOLVER}/01/{GTIN}/21/{serial}"),
        format!("{RESOLVER}/01/{GTIN}/10/LOT-X-0001/21/{serial}"),
    ] {
        assert_eq!(resolve(&store, &label).await, [model.id], "{label}");
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

/// Below batch level the serial alone resolves the label, so a lot the record
/// holds is not printed — only a batch-level carrier, which identifies the
/// lot, carries one.
#[test]
fn only_a_batch_level_carrier_prints_the_lot() {
    for level in [Some(Granularity::Item), None] {
        let mut passport = at(level);
        passport.batch_id = Some("LOT-A".into());
        let label = carrier(&passport);
        let link = DigitalLink::parse(&label).expect("parses");
        assert_eq!(link.batch(), None, "{label}");
        assert!(!label.contains("LOT-A"), "{label}");
    }
}

/// A batch-level passport's lot is printed, so a lot GS1 would refuse is
/// refused before it is printed — and a batch-level passport with no lot has
/// no carrier, rather than one that falls back to a serial.
#[test]
fn a_lot_gs1_would_reject_is_not_printed() {
    let with = |batch: Option<&str>| {
        let mut passport = at(Some(Granularity::Batch));
        passport.batch_id = batch.map(str::to_owned);
        build_qr_url(RESOLVER, &passport)
    };
    assert!(matches!(with(None), Err(DigitalLinkError::EmptyValue(code)) if code == "10"));
    assert!(matches!(with(Some("")), Err(DigitalLinkError::EmptyValue(code)) if code == "10"));
    assert!(matches!(
        with(Some("LOT A")),
        Err(DigitalLinkError::OutsideCset82 { code, character: ' ' }) if code == "10"
    ));
    assert!(matches!(
        with(Some("123456789012345678901")),
        Err(DigitalLinkError::ValueTooLong { code, max_len: 20, actual: 21 }) if code == "10"
    ));
}

/// A product group this build has no typed variant for still names the product
/// it identifies, so its passport prints a carrier and resolves through it like
/// any other — adding a product group stays a data change, not a release.
#[tokio::test]
async fn an_untyped_product_group_prints_a_carrier_that_resolves() {
    let store = Store::default();
    let mut passport = gs1_battery();
    passport.product_group = ProductGroup::Other("photovoltaic".into());
    passport.product_group_data = ProductGroupData::other(serde_json::json!({
        "productGroup": "photovoltaic",
        "productIdentifier": { "scheme": "gs1", "gtin": GTIN },
    }));
    store.create(passport.clone()).await.expect("stored");

    let label = carrier(&passport);
    assert_eq!(
        label,
        format!(
            "{RESOLVER}/01/{GTIN}/21/{}",
            passport.id.default_carrier_serial()
        )
    );
    assert_eq!(resolve(&store, &label).await, [passport.id]);
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

/// The domain restates GS1's AI 10 and AI 21 rules because it cannot read the
/// dictionary this crate vendors. Held against that dictionary here, where
/// both are in reach, so the restatements cannot drift from their source.
#[test]
fn the_restated_ai_10_and_ai_21_rules_match_the_dictionary() {
    for (ai, max, check) in [
        ("10", MAX_GS1_LOT_CHARS, "check_gs1_lot"),
        ("21", MAX_GS1_SERIAL_CHARS, "check_gs1_serial"),
    ] {
        let spec = ai_spec(ai).expect("in the dictionary");
        assert_eq!(spec.max_len, max, "AI {ai}");
        assert_eq!(spec.min_len, 1, "`{check}` refuses an empty value");
        assert!(spec.cset_82, "`{check}` applies CSET 82");
    }
}
