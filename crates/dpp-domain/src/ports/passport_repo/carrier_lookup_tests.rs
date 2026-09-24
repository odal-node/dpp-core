//! What a printed data carrier resolves to.
//!
//! The first test is the defect this lookup exists for, kept as a regression:
//! the lookup a printed label went through answered **nothing** for a model,
//! a lot and a unit passport alike, because the label's AI 21 was never the
//! field the lookup compared.

use std::borrow::Cow;

use super::port::PassportRepository;
use super::tests::{InMemoryRepo, fixture_identifier, identified_passport};
use crate::Granularity;
use crate::identifier::ProductIdentifier;
use crate::passport::{CarrierQualifier, Passport};
use crate::status::PassportStatus;

fn serial(value: &str) -> CarrierQualifier<'_> {
    CarrierQualifier::Serial(Cow::Borrowed(value))
}

fn at(level: Granularity, name: &str) -> Passport {
    let mut passport = identified_passport(name);
    passport.granularity = Some(level);
    passport
}

async fn names(repo: &InMemoryRepo, qualifier: &CarrierQualifier<'_>) -> Vec<String> {
    let mut found: Vec<_> = repo
        .find_by_carrier(&fixture_identifier(), qualifier)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.product_name)
        .collect();
    found.sort();
    found
}

/// Every level resolves through the serial its own carrier prints, and to
/// itself alone.
#[tokio::test]
async fn every_level_resolves_through_its_own_carrier_serial() {
    let repo = InMemoryRepo::default();

    let model = repo.create(identified_passport("Model")).await.unwrap();

    let mut lot = identified_passport("Lot");
    lot.batch_id = Some("LOT-A".into());
    let lot = repo.create(lot).await.unwrap();

    let mut unit = identified_passport("Unit");
    unit.batch_id = Some("LOT-A".into());
    unit.serial_number = Some("SN-0001".into());
    let unit = repo.create(unit).await.unwrap();

    let id = fixture_identifier();
    for passport in [&model, &lot, &unit] {
        let found = repo
            .find_by_carrier(&id, &serial(&passport.effective_carrier_serial()))
            .await
            .unwrap();
        assert_eq!(found.len(), 1, "{}", passport.product_name);
        assert_eq!(found[0].id, passport.id, "{}", passport.product_name);
    }
}

/// Each level's own carrier finds it and nothing else under the same GTIN: the
/// model label does not reach a batch or a unit, a lot label does not reach
/// the units of that lot or another lot, and a unit's serial names one unit.
#[tokio::test]
async fn each_level_resolves_through_its_own_carrier_and_to_itself_alone() {
    let repo = InMemoryRepo::default();

    let model = repo.create(at(Granularity::Model, "Model")).await.unwrap();

    let mut lot_a = at(Granularity::Batch, "Lot A");
    lot_a.batch_id = Some("LOT-A".into());
    let lot_a = repo.create(lot_a).await.unwrap();

    let mut lot_b = at(Granularity::Batch, "Lot B");
    lot_b.batch_id = Some("LOT-B".into());
    repo.create(lot_b).await.unwrap();

    let mut unit = at(Granularity::Item, "Unit");
    unit.batch_id = Some("LOT-A".into());
    let unit = repo.create(unit).await.unwrap();

    for passport in [&model, &lot_a, &unit] {
        let qualifier = passport.carrier_qualifier().expect("a printable level");
        assert_eq!(
            names(&repo, &qualifier).await,
            [passport.product_name.as_str()],
            "{qualifier:?}"
        );
    }
}

/// A passport whose level is not stated prints a serial, so a GTIN-only label
/// does not name it — only a passport that states model level answers one.
#[tokio::test]
async fn a_model_label_names_only_a_passport_that_states_model_level() {
    let repo = InMemoryRepo::default();
    let mut unstated = identified_passport("Unstated");
    unstated.granularity = None;
    repo.create(unstated).await.unwrap();

    assert!(names(&repo, &CarrierQualifier::Model).await.is_empty());

    repo.create(at(Granularity::Model, "Model")).await.unwrap();
    assert_eq!(names(&repo, &CarrierQualifier::Model).await, ["Model"]);
}

/// A serial label outlives the rules it was printed under: one printed when
/// every carrier carried a serial still resolves to a passport that now states
/// model level, because a carrier serial names one passport at any level.
#[tokio::test]
async fn a_serial_label_resolves_a_passport_at_any_level() {
    let repo = InMemoryRepo::default();
    let model = repo.create(at(Granularity::Model, "Model")).await.unwrap();

    let found = repo
        .find_by_carrier(
            &fixture_identifier(),
            &serial(&model.effective_carrier_serial()),
        )
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, model.id);
}

/// An attributed serial is the one the label carries, so it resolves and the
/// default it replaced does not.
#[tokio::test]
async fn an_attributed_serial_replaces_the_default() {
    let repo = InMemoryRepo::default();
    let mut unit = identified_passport("Unit");
    unit.carrier_serial = Some("SN-2026/00042".into());
    let unit = repo.create(unit).await.unwrap();

    let id = fixture_identifier();
    let found = repo
        .find_by_carrier(&id, &serial("SN-2026/00042"))
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, unit.id);

    assert!(
        repo.find_by_carrier(&id, &serial(&unit.id.default_carrier_serial()))
            .await
            .unwrap()
            .is_empty()
    );
}

/// A label whose GTIN is not the record's is not that record's label.
#[tokio::test]
async fn a_serial_under_another_identifier_finds_nothing() {
    let repo = InMemoryRepo::default();
    let unit = repo.create(identified_passport("Unit")).await.unwrap();

    let other = ProductIdentifier::gs1(crate::Gtin::parse("00012345600012").expect("a GTIN"));
    assert!(
        repo.find_by_carrier(&other, &serial(&unit.effective_carrier_serial()))
            .await
            .unwrap()
            .is_empty()
    );
}

/// After an amendment the label on the object is unchanged, so a successor
/// that carries the serial forward is named by it too — and a withdrawn
/// predecessor is still found, with its status, so the route can say so.
#[tokio::test]
async fn a_serial_carried_forward_names_the_whole_chain() {
    let repo = InMemoryRepo::default();

    let mut predecessor = identified_passport("v1");
    predecessor.status = PassportStatus::Superseded;
    let predecessor = repo.create(predecessor).await.unwrap();

    let mut successor = identified_passport("v2");
    successor.supersedes_id = Some(predecessor.id);
    successor.carrier_serial = Some(predecessor.effective_carrier_serial().into_owned());
    successor.status = PassportStatus::Published;
    let successor = repo.create(successor).await.unwrap();

    let mut found = repo
        .find_by_carrier(
            &fixture_identifier(),
            &serial(&predecessor.effective_carrier_serial()),
        )
        .await
        .unwrap();
    found.sort_by_key(|p| p.product_name.clone());
    let ids: Vec<_> = found.iter().map(|p| (p.id, p.status.clone())).collect();
    assert_eq!(
        ids,
        [
            (predecessor.id, PassportStatus::Superseded),
            (successor.id, PassportStatus::Published)
        ]
    );
}
