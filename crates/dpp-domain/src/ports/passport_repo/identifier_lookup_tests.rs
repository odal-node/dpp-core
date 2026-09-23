//! What a product identifier resolves to.
//!
//! Split from `tests.rs` on the 400-line rule, and they belong together: all
//! three are about the lookup that replaced the two by-GTIN methods.

use super::port::PassportRepository;
use super::tests::{InMemoryRepo, fixture_identifier, identified_passport};
use crate::identifier::ProductIdentifier;
use crate::status::PassportStatus;

/// A withdrawn passport stays reachable, and its status comes back with it.
///
/// The distinction the removed `find_published_by_gtin` could not express: its
/// `None` meant "unknown identifier" and "withdrawn" at once, so a scanned-code
/// route could not tell `404` from the `410 Gone` a consumer holding the
/// product needs to see. No status filter here, so the caller can.
#[tokio::test]
async fn a_suspended_passport_is_still_found_and_carries_its_status() {
    let repo = InMemoryRepo::default();
    let mut p = identified_passport("Suspended battery");
    p.status = PassportStatus::Suspended;
    let created = repo.create(p).await.unwrap();

    let found = repo
        .find_by_identifier(&fixture_identifier(), None, None)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, created.id);
    assert_eq!(found[0].status, PassportStatus::Suspended);
}

/// 🚨 The defect the old signature hid: an identifier names N passports.
///
/// `Granularity` admits `Model`, `Batch` and `Item`, so one GTIN can have a
/// model record, one per production run and one per unit. The removed methods
/// returned "the first" of those with no ordering defined — one arbitrary row.
#[tokio::test]
async fn one_identifier_can_name_several_passports() {
    let repo = InMemoryRepo::default();

    let mut model = identified_passport("Model");
    model.status = PassportStatus::Published;
    let model = repo.create(model).await.unwrap();

    let mut lot_a = identified_passport("Lot A");
    lot_a.batch_id = Some("LOT-A".into());
    lot_a.status = PassportStatus::Published;
    let lot_a = repo.create(lot_a).await.unwrap();

    let mut unit = identified_passport("Unit 1");
    unit.batch_id = Some("LOT-A".into());
    unit.serial_number = Some("SN-0001".into());
    unit.status = PassportStatus::Published;
    let unit = repo.create(unit).await.unwrap();

    let id = fixture_identifier();

    // Unnarrowed: the model-level record only, because the batch and unit
    // records carry a batch and this asked for one with none.
    let at_model = repo.find_by_identifier(&id, None, None).await.unwrap();
    assert_eq!(at_model.len(), 1);
    assert_eq!(at_model[0].id, model.id);

    // Narrowed to the run, and then to the unit by its manufacturer's serial.
    let at_batch = repo
        .find_by_identifier(&id, Some("LOT-A"), None)
        .await
        .unwrap();
    assert_eq!(at_batch.len(), 1);
    assert_eq!(at_batch[0].id, lot_a.id);

    let at_unit = repo
        .find_by_identifier(&id, Some("LOT-A"), Some("SN-0001"))
        .await
        .unwrap();
    assert_eq!(at_unit.len(), 1);
    assert_eq!(at_unit[0].id, unit.id);

    // All three are real, distinct, published passports for one identifier.
    assert!(
        model.id != lot_a.id && lot_a.id != unit.id && model.id != unit.id,
        "three distinct published passports for one identifier"
    );
}

/// A manufacturer's serial finds its unit whether or not the caller knows the
/// lot. GTIN and serial already identify one unit; requiring the batch too would
/// miss every unit recorded with a lot whenever the lot was not to hand.
#[tokio::test]
async fn a_serial_finds_its_unit_whatever_batch_is_asked_for() {
    let repo = InMemoryRepo::default();
    let mut unit = identified_passport("Unit 1");
    unit.batch_id = Some("LOT-A".into());
    unit.serial_number = Some("SN-0001".into());
    let unit = repo.create(unit).await.unwrap();

    let id = fixture_identifier();
    for batch in [None, Some("LOT-A"), Some("LOT-B")] {
        let found = repo
            .find_by_identifier(&id, batch, Some("SN-0001"))
            .await
            .unwrap();
        assert_eq!(found.len(), 1, "batch {batch:?}");
        assert_eq!(found[0].id, unit.id);
    }

    // And without a serial, a lot names the run's own record, not its units.
    assert!(
        repo.find_by_identifier(&id, Some("LOT-A"), None)
            .await
            .unwrap()
            .is_empty()
    );
}

/// An identifier nothing carries is an empty result, not a wrong one.
#[tokio::test]
async fn an_unknown_identifier_finds_nothing() {
    let repo = InMemoryRepo::default();
    let mut p = identified_passport("Known");
    p.status = PassportStatus::Published;
    repo.create(p).await.unwrap();

    let other = ProductIdentifier::gs1(crate::Gtin::parse("00012345600012").expect("a GTIN"));
    assert!(
        repo.find_by_identifier(&other, None, None)
            .await
            .unwrap()
            .is_empty()
    );
}
