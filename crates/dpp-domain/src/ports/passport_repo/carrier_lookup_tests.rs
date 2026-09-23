//! What a printed data carrier resolves to.
//!
//! The first test is the defect this lookup exists for, kept as a regression:
//! the lookup a printed label went through answered **nothing** for a model,
//! a lot and a unit passport alike, because the label's AI 21 was never the
//! field the lookup compared.

use super::port::PassportRepository;
use super::tests::{InMemoryRepo, fixture_identifier, identified_passport};
use crate::identifier::ProductIdentifier;
use crate::status::PassportStatus;

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
            .find_by_carrier_serial(&id, &passport.effective_carrier_serial())
            .await
            .unwrap();
        assert_eq!(found.len(), 1, "{}", passport.product_name);
        assert_eq!(found[0].id, passport.id, "{}", passport.product_name);
    }
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
        .find_by_carrier_serial(&id, "SN-2026/00042")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, unit.id);

    assert!(
        repo.find_by_carrier_serial(&id, &unit.id.default_carrier_serial())
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
        repo.find_by_carrier_serial(&other, &unit.effective_carrier_serial())
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
        .find_by_carrier_serial(
            &fixture_identifier(),
            &predecessor.effective_carrier_serial(),
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
