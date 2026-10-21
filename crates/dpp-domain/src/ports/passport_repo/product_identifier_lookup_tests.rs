//! What an identifier resolves to at every level, with nothing to narrow by.
//!
//! The first test is the gap this lookup exists for, kept as a regression: a
//! passport identified by a link or a DID that carries a batch or a serial was
//! found by no method, because the unnarrowed identifier lookup answers the
//! model-level record only and the carrier lookup needs a qualifier.

use super::port::PassportRepository;
use super::tests::{InMemoryRepo, fixture_identifier, identified_passport};
use crate::Granularity;
use crate::identifier::ProductIdentifier;
use crate::passport::Passport;
use crate::status::PassportStatus;

fn link(path: &str) -> ProductIdentifier {
    ProductIdentifier::identification_link(&format!("https://id.example.com/{path}"))
        .expect("an identification link")
}

fn did(path: &str) -> ProductIdentifier {
    ProductIdentifier::did(&format!("did:web:id.example.com:{path}")).expect("a DID")
}

/// A passport under `identifier` at `level`: a batch carries a lot, and an item
/// carries a lot and a serial number.
fn at(level: Granularity, identifier: &ProductIdentifier, name: &str) -> Passport {
    let mut passport = identified_passport(name);
    let Some(crate::ProductGroupData::Battery(data)) = passport.product_group_data.as_mut() else {
        panic!("identified_passport carries battery data");
    };
    data.product_identifier = identifier.clone();
    passport.granularity = Some(level);
    if level != Granularity::Model {
        passport.batch_id = Some("LOT-A".into());
    }
    if level == Granularity::Item {
        passport.serial_number = Some("SN-0001".into());
    }
    passport
}

async fn found(repo: &InMemoryRepo, identifier: &ProductIdentifier) -> Vec<String> {
    let mut names: Vec<_> = repo
        .find_by_product_identifier(identifier)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.product_name)
        .collect();
    names.sort();
    names
}

/// 🚨 A link and a DID each name one passport at whatever level it is, and the
/// identifier alone finds it. The unnarrowed `find_by_identifier` misses the
/// batch and the item, which is the gap this method closes.
#[tokio::test]
async fn a_link_or_a_did_finds_its_passport_at_every_level() {
    let repo = InMemoryRepo::default();
    let levels = [
        (Granularity::Model, "model"),
        (Granularity::Batch, "batch"),
        (Granularity::Item, "item"),
    ];

    for (scheme, identify) in [("link", link as fn(&str) -> _), ("did", did)] {
        for (level, slug) in levels {
            let identifier = identify(slug);
            let name = format!("{scheme} {slug}");
            repo.create(at(level, &identifier, &name)).await.unwrap();

            assert_eq!(found(&repo, &identifier).await, [name.as_str()]);

            let unnarrowed = repo
                .find_by_identifier(&identifier, None, None)
                .await
                .unwrap();
            assert_eq!(
                unnarrowed.len(),
                usize::from(level == Granularity::Model),
                "{name}: the unnarrowed identifier lookup answers the model record only"
            );
        }
    }
}

/// An identifier no passport carries finds nothing, even beside passports at
/// every level under other identifiers.
#[tokio::test]
async fn another_identifier_finds_nothing() {
    let repo = InMemoryRepo::default();
    repo.create(at(Granularity::Item, &did("item"), "Item"))
        .await
        .unwrap();
    repo.create(at(Granularity::Batch, &link("batch"), "Batch"))
        .await
        .unwrap();

    assert!(found(&repo, &did("other")).await.is_empty());
    assert!(found(&repo, &link("other")).await.is_empty());
}

/// An amendment carries the identifier forward, so a superseded record and its
/// successor both come back, each with its status, for the caller to choose.
#[tokio::test]
async fn a_superseded_record_and_its_successor_both_come_back() {
    let repo = InMemoryRepo::default();
    let identifier = did("item");

    let mut predecessor = at(Granularity::Item, &identifier, "v1");
    predecessor.status = PassportStatus::Superseded;
    let predecessor = repo.create(predecessor).await.unwrap();

    let mut successor = at(Granularity::Item, &identifier, "v2");
    successor.supersedes_id = Some(predecessor.id);
    successor.status = PassportStatus::Published;
    let successor = repo.create(successor).await.unwrap();

    let mut chain = repo.find_by_product_identifier(&identifier).await.unwrap();
    chain.sort_by_key(|p| p.product_name.clone());
    let ids: Vec<_> = chain.iter().map(|p| (p.id, p.status.clone())).collect();
    assert_eq!(
        ids,
        [
            (predecessor.id, PassportStatus::Superseded),
            (successor.id, PassportStatus::Published)
        ]
    );
}

/// For a GTIN, which is narrowed by its qualifiers elsewhere, this is every
/// passport of the product: the model, each run and each unit.
#[tokio::test]
async fn a_gtin_finds_every_passport_of_the_product() {
    let repo = InMemoryRepo::default();
    let gtin = fixture_identifier();
    for (level, name) in [
        (Granularity::Model, "Model"),
        (Granularity::Batch, "Lot"),
        (Granularity::Item, "Unit"),
    ] {
        repo.create(at(level, &gtin, name)).await.unwrap();
    }

    assert_eq!(found(&repo, &gtin).await, ["Lot", "Model", "Unit"]);
}
