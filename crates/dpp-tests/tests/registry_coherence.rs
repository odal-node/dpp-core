//! A registration checked against the passport it registers — IR (EU) 2026/1778
//! Art. 8(7)(b)–(c).
//!
//! `RegistrationPayload::check_coherence` lives in `dpp-registry`, which has no
//! passport fixtures. These build the passport the way a consumer holds it and
//! the registration the way an adapter builds one, from the port request, so
//! what is checked is the composition a real registration goes through.

use chrono::Utc;
use dpp_domain::catalog::Granularity as LevelInTheAct;
use dpp_domain::identifier::ProductIdentifier as SchemeIdentifier;
use dpp_domain::ports::registry_sync::{
    RegisteringOperator, RegistrationGranularity, RegistrationRequest,
};
use dpp_domain::{
    BatteryChemistry, BatteryData, BatteryType, Gtin, Passport, PassportStatus, ProductGroup,
    ProductGroupData,
};
use dpp_registry::{
    FacilityIdentifier, Granularity, OperatorIdentifier, ProductIdentifier, ProductItemIdentifier,
    RegistrationLevel, RegistrationPayload, RegistryValidationError,
};
use dpp_tests::fixtures::base_passport;

const CARRIER: &str = "https://id.example.com/01/09506000134352/21/SN0001";

/// Battery data carrying a model identifier, the one product group whose
/// typed data names one. Built from the five fields the type requires, with
/// the enums written by serde so no wire spelling is guessed here.
fn battery_data(model: &str) -> BatteryData {
    serde_json::from_value(serde_json::json!({
        "productIdentifier": SchemeIdentifier::gs1(Gtin::parse("09506000134352").unwrap()),
        "batteryChemistry": BatteryChemistry::Lfp,
        "nominalVoltageV": 3.2,
        "nominalCapacityAh": 100.0,
        "batteryType": BatteryType::Ev,
        "batteryModelId": model,
    }))
    .expect("the five required fields and a model make battery data")
}

fn published_battery() -> Passport {
    let mut passport = base_passport(
        ProductGroup::Battery,
        ProductGroupData::Battery(Box::new(battery_data("VD-4680"))),
        "2.8.0",
    );
    passport.status = PassportStatus::Published;
    passport.published_at = Some(Utc::now());
    // An LEI, because a registration carries an identifier with an EN 18219
    // clause 6 route and a VAT number has none.
    passport.operator_identifier = Some("529900T8BM49AURSDO55".into());
    passport.qr_code_url = Some(CARRIER.into());
    passport.carrier_serial = Some("SN0001".into());
    passport.batch_id = Some("LOT-7".into());
    passport.facility = Some(dpp_domain::passport::FacilitySnapshot {
        scheme: "gln".into(),
        value: "4012345000009".into(),
        name: "Plant 3".into(),
        country: "DE".into(),
        address: None,
        identifier: None,
    });
    passport
}

/// The registration an adapter builds from the port request, field for field.
fn registration_for(
    passport: &Passport,
    granularity: RegistrationGranularity,
) -> RegistrationPayload {
    let request = RegistrationRequest::from_published_passport(
        passport,
        RegisteringOperator {
            legal_name: "Volt GmbH",
            country: "DE",
            identifier_scheme: "lei",
        },
        granularity,
    )
    .expect("the fixture passport carries everything a registration needs");

    let mut level = RegistrationLevel::new(match request.granularity {
        RegistrationGranularity::Model => Granularity::Model,
        RegistrationGranularity::Batch => Granularity::Batch,
        RegistrationGranularity::Item => Granularity::Item,
    });
    if let Some(model) = &request.model_id {
        level = level.with_model(model.clone());
    }
    if let Some(batch) = &request.batch_id {
        level = level.with_batch(batch.clone());
    }
    RegistrationPayload {
        passport_id: request.passport_id.0,
        product_id: ProductIdentifier::try_from(
            request
                .product_identifier
                .as_ref()
                .expect("the constructor refuses a passport that identifies nothing"),
        )
        .expect("a GTIN maps"),
        level,
        item_id: request
            .item_serial
            .as_ref()
            .map(|serial| ProductItemIdentifier {
                scheme: "serial".into(),
                value: serial.clone(),
                batch_id: request.batch_id.clone(),
            }),
        facility_id: FacilityIdentifier {
            scheme: "gln".into(),
            value: request.facility_identifier.clone(),
            name: Some("Plant 3".into()),
            country: "DE".into(),
            address: None,
        },
        operator_id: OperatorIdentifier {
            scheme: request.operator_identifier_scheme.clone(),
            value: request.operator_identifier.clone(),
            name: request.operator_name.clone(),
            country: request.country_code.clone(),
            did: None,
        },
        // The registry's code for the group, as an adapter must send it.
        product_group: dpp_registry::registry_product_group(&passport.product_group)
            .expect("batteries have a registry code")
            .to_owned(),
        schema_version: request.schema_version.clone(),
        digital_link_url: request.data_carrier_uri.clone(),
        published_at: Utc::now(),
        jws_signature: None,
        commodity_code: request.commodity_code.clone(),
        backup_url: None,
        service_provider: None,
    }
}

fn incoherent_field(result: Result<(), RegistryValidationError>) -> (&'static str, Option<String>) {
    match result {
        Err(RegistryValidationError::Incoherent {
            field, registered, ..
        }) => (field, registered),
        other => panic!("expected an incoherence, got {other:?}"),
    }
}

/// The baseline every other case breaks one thing in: a registration built
/// from its passport through the port agrees with it, at every level.
#[test]
fn a_registration_built_from_its_passport_coheres_at_every_level() {
    let passport = published_battery();
    for granularity in [
        RegistrationGranularity::Item,
        RegistrationGranularity::Batch,
        RegistrationGranularity::Model,
    ] {
        let payload = registration_for(&passport, granularity);
        assert!(payload.validate().is_ok(), "{:?}", payload.validate());
        assert_eq!(
            payload.check_coherence(&passport),
            Ok(()),
            "{granularity:?}"
        );
    }
}

/// Art. 8(7)(b): a registered value the passport states differently.
#[test]
fn a_value_the_passport_states_differently_is_incoherent() {
    let passport = published_battery();
    let item = || registration_for(&passport, RegistrationGranularity::Item);

    let mut carrier = item();
    carrier.digital_link_url = "https://id.example.com/01/09506000134352/21/OTHER".into();
    assert_eq!(
        incoherent_field(carrier.check_coherence(&passport)).0,
        "digitalLinkUrl"
    );

    let mut model = item();
    model.level.model_id = Some("VD-9999".into());
    assert_eq!(
        incoherent_field(model.check_coherence(&passport)),
        ("level.modelId", Some("VD-9999".into()))
    );

    let mut operator = item();
    operator.operator_id.value = "DE999999999".into();
    assert_eq!(
        incoherent_field(operator.check_coherence(&passport)).0,
        "operatorId"
    );

    let mut group = item();
    group.product_group = "textile".into();
    assert_eq!(
        incoherent_field(group.check_coherence(&passport)).0,
        "productGroup"
    );

    // Our own name for the group is not the registry's: a battery filed as
    // `battery` names a group the registry does not list.
    let mut our_name = item();
    our_name.product_group = "battery".into();
    assert_eq!(
        incoherent_field(our_name.check_coherence(&passport)),
        ("productGroup", Some("battery".into()))
    );
}

/// Art. 8(4): an item-level registration links the model and the batch the
/// passport shows exist. Leaving one out is reported with nothing registered.
#[test]
fn a_link_the_passport_shows_exists_must_be_made() {
    let passport = published_battery();

    let mut no_model = registration_for(&passport, RegistrationGranularity::Item);
    no_model.level.model_id = None;
    assert_eq!(
        incoherent_field(no_model.check_coherence(&passport)),
        ("level.modelId", None)
    );

    let mut no_batch = registration_for(&passport, RegistrationGranularity::Item);
    no_batch.level.batch_id = None;
    assert_eq!(
        incoherent_field(no_batch.check_coherence(&passport)),
        ("level.batchId", None)
    );

    // Art. 8(5) links only the model at batch level, and a model-level
    // registration covers every batch, so neither requires a batch link.
    let mut model_level = registration_for(&passport, RegistrationGranularity::Model);
    model_level.level.batch_id = None;
    assert_eq!(model_level.check_coherence(&passport), Ok(()));
}

/// A passport's public view withholds its batch unless its product group opens
/// it. A value the registry cannot read is not one it can contradict, so a
/// passport that states no batch is not compared on it.
#[test]
fn a_value_the_passport_does_not_state_is_not_compared() {
    let mut passport = published_battery();
    let registered = registration_for(&passport, RegistrationGranularity::Item);
    passport.batch_id = None;
    assert_eq!(registered.check_coherence(&passport), Ok(()));
}

/// Art. 8(7)(c): where the passport states its level, it is the one registered.
/// Where it states none — every product group today, since no adopted act fixes
/// one — there is nothing to conform to.
#[test]
fn a_level_the_passport_states_is_the_level_registered() {
    let mut passport = published_battery();
    let item = registration_for(&passport, RegistrationGranularity::Item);

    passport.granularity = Some(LevelInTheAct::Batch);
    assert_eq!(
        incoherent_field(item.check_coherence(&passport)),
        ("level", Some("item".into()))
    );

    passport.granularity = Some(LevelInTheAct::Item);
    assert_eq!(item.check_coherence(&passport), Ok(()));
}
