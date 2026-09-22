//! The passport repository contract, exercised against an in-memory double.

use super::port::*;
use async_trait::async_trait;

use crate::error::DppError;
use crate::identifier::ProductIdentifier;
use crate::passport::ManufacturerInfo;
use crate::product_group::ProductGroup;
use crate::{
    passport::{Passport, PassportId},
    product::ProductIdentity,
    status::PassportStatus,
};
use std::collections::HashMap;
use std::sync::Mutex;

/// Minimal in-memory repo to exercise the trait's **default** method bodies
/// (`patch_fields`, `create_batch`, `update_batch`). Only the methods those
/// defaults call are functional; the rest satisfy the signature.
#[derive(Default)]
pub(super) struct InMemoryRepo {
    store: Mutex<HashMap<PassportId, Passport>>,
}

#[async_trait]
impl PassportRepository for InMemoryRepo {
    async fn create(&self, passport: Passport) -> Result<Passport, DppError> {
        self.store
            .lock()
            .unwrap()
            .insert(passport.id, passport.clone());
        Ok(passport)
    }
    async fn find_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError> {
        Ok(self.store.lock().unwrap().get(&id).cloned())
    }
    async fn find_published_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError> {
        self.find_by_id(id).await
    }
    async fn find_by_id_any_status(&self, id: PassportId) -> Result<Option<Passport>, DppError> {
        self.find_by_id(id).await
    }
    async fn update(&self, passport: Passport) -> Result<Passport, DppError> {
        self.store
            .lock()
            .unwrap()
            .insert(passport.id, passport.clone());
        Ok(passport)
    }
    async fn update_status(
        &self,
        id: PassportId,
        status: PassportStatus,
    ) -> Result<Passport, DppError> {
        let mut g = self.store.lock().unwrap();
        let mut p = g
            .get(&id)
            .cloned()
            .ok_or(DppError::NotFound(id.to_string()))?;
        p.status = status;
        g.insert(id, p.clone());
        Ok(p)
    }
    async fn list(
        &self,
        status: Option<PassportStatus>,
        _q: Option<&str>,
        _facility_id: Option<&str>,
        _limit: u32,
        _offset: u32,
    ) -> Result<Vec<Passport>, DppError> {
        // 🚨 This ignored `status` entirely, which made it a double that does
        // not model the contract it stands in for: every default implementation
        // written in terms of `list(Some(..))` was exercised against a store
        // that answered with everything. `None` is every status, matching the
        // `facility_id` reading on the trait.
        Ok(self
            .store
            .lock()
            .unwrap()
            .values()
            .filter(|p| status.as_ref().is_none_or(|s| p.status == *s))
            .cloned()
            .collect())
    }
    async fn count(
        &self,
        _status: Option<PassportStatus>,
        _facility_id: Option<&str>,
    ) -> Result<u64, DppError> {
        Ok(self.store.lock().unwrap().len() as u64)
    }
}

/// The identifier every fixture below carries — the GTIN on
/// `sample_battery_data`, so a lookup has something real to match.
pub(super) fn fixture_identifier() -> ProductIdentifier {
    ProductIdentifier::gs1(crate::Gtin::parse("09506000134352").expect("a GTIN"))
}

/// A draft passport that carries product group data, and therefore an
/// identifier. `sample_passport` deliberately has none.
pub(super) fn identified_passport(name: &str) -> Passport {
    Passport {
        product_group: crate::ProductGroup::Battery,
        product_group_data: Some(crate::ProductGroupData::Battery(Box::new(
            crate::test_support::sample_battery_data(),
        ))),
        ..draft_passport(name)
    }
}

fn draft_passport(name: &str) -> Passport {
    Passport {
        product_name: name.into(),
        manufacturer: ManufacturerInfo {
            name: "Brand".into(),
            address: "Berlin, DE".into(),
            country: None,
            did_web_url: None,
            registered_trade_name: None,
            electronic_address: None,
        },
        schema_version: "1.1.0".into(),
        ..crate::test_support::sample_passport()
    }
}

#[tokio::test]
async fn default_patch_fields_merges_delta() {
    let repo = InMemoryRepo::default();
    let p = repo.create(draft_passport("Original")).await.unwrap();

    let patched = repo
        .patch_fields(p.id, serde_json::json!({ "productName": "Renamed" }))
        .await
        .unwrap();
    assert_eq!(patched.product_name, "Renamed");
    // Untouched fields are preserved.
    assert_eq!(patched.id, p.id);
}

#[tokio::test]
async fn default_patch_fields_rejects_protected_fields() {
    let repo = InMemoryRepo::default();
    let p = repo.create(draft_passport("Original")).await.unwrap();

    // A delta that tries to escape the state machine / forge integrity fields.
    let err = repo
        .patch_fields(
            p.id,
            serde_json::json!({
                "status": "active",
                "retentionLocked": false,
                "jwsSignature": "forged",
            }),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, DppError::Validation(_)), "got: {err:?}");

    // The passport must be untouched — still a retention-unlocked draft.
    let stored = repo.find_by_id(p.id).await.unwrap().unwrap();
    assert_eq!(stored.status, PassportStatus::Draft);
    assert!(!stored.retention_locked);
    assert!(stored.jws_signature.is_none());
}

#[tokio::test]
async fn default_patch_fields_rejects_operator_and_facility() {
    let repo = InMemoryRepo::default();
    let p = repo.create(draft_passport("Original")).await.unwrap();

    // operatorIdentifier changes belong to RegistrySyncPort::notify_transfer;
    // facility is a create-time snapshot. Neither is patchable.
    let err = repo
        .patch_fields(
            p.id,
            serde_json::json!({
                "operatorIdentifier": "did:web:new-owner.example.com",
                "facility": {
                    "scheme": "national",
                    "value": "FAC-DE-999",
                    "country": "DE",
                },
            }),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, DppError::Validation(_)), "got: {err:?}");

    let stored = repo.find_by_id(p.id).await.unwrap().unwrap();
    assert!(stored.operator_identifier.is_none());
    assert!(stored.facility.is_none());
}

/// Lineage edges are create-time by construction and live in the signed
/// public view, so a free-form patch must not reach them: a second-life
/// passport is issued as a new record, and a bill-of-materials change is a
/// new passport version. See `docs/architecture/PRODUCT-LINEAGE.md`.
#[tokio::test]
async fn default_patch_fields_rejects_lineage_edges() {
    let repo = InMemoryRepo::default();
    let p = repo.create(draft_passport("Original")).await.unwrap();

    for delta in [
        serde_json::json!({
            "derivedFrom": [{
                "reference": {
                    "uri": "https://id.example.com/dpp/other",
                    "publicJwsHash": "00",
                },
                "operation": "repurposing",
            }]
        }),
        serde_json::json!({
            "componentRefs": [{
                "reference": {
                    "uri": "https://id.example.com/dpp/cell",
                    "publicJwsHash": "00",
                },
                "quantity": { "value": 2.0 },
                "role": "cell",
            }]
        }),
    ] {
        let err = repo.patch_fields(p.id, delta).await.unwrap_err();
        assert!(matches!(err, DppError::Validation(_)), "got: {err:?}");
    }

    let stored = repo.find_by_id(p.id).await.unwrap().unwrap();
    assert!(stored.derived_from.is_empty());
    assert!(stored.component_refs.is_empty());
}

#[tokio::test]
async fn default_patch_fields_unknown_id_is_not_found() {
    let repo = InMemoryRepo::default();
    let err = repo
        .patch_fields(PassportId::new(), serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, DppError::NotFound(_)));
}

#[tokio::test]
async fn default_find_by_identity_matches_across_draft_and_published() {
    use crate::product_group::ProductGroupData;

    let repo = InMemoryRepo::default();
    let mut p = draft_passport("Battery A");
    p.product_group = ProductGroup::Battery;
    p.product_group_data = Some(ProductGroupData::Battery(Box::new(
        crate::test_support::sample_battery_data(),
    )));
    p.batch_id = Some("BATCH-1".into());
    let created = repo.create(p).await.unwrap();

    let identity = ProductIdentity {
        product_group: ProductGroup::Battery,
        identifier: "09506000134352".into(),
        batch_id: Some("BATCH-1".into()),
        serial_number: None,
    };
    let found = repo.find_by_identity(&identity).await.unwrap();
    assert_eq!(found.map(|p| p.id), Some(created.id));

    let no_match = ProductIdentity {
        product_group: ProductGroup::Battery,
        identifier: "00000000000000".into(),
        batch_id: None,
        serial_number: None,
    };
    assert!(repo.find_by_identity(&no_match).await.unwrap().is_none());
}

#[tokio::test]
async fn default_create_and_update_batch_run_sequentially() {
    let repo = InMemoryRepo::default();
    let created = repo
        .create_batch(vec![draft_passport("A"), draft_passport("B")])
        .await;
    assert_eq!(created.len(), 2);
    assert!(created.iter().all(|r| r.is_ok()));

    let mut a = created[0].as_ref().unwrap().clone();
    a.product_name = "A2".into();
    let updated = repo.update_batch(vec![a]).await;
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].as_ref().unwrap().product_name, "A2");
}

// ── Resolving forward through an amendment ───────────────────────────────────
//
// `supersedes_id` points backwards, so the successor is only reachable by
// querying for it. These exercise the trait's default bodies: the one-hop
// lookup, the walk to the head, and the three shapes that have no answer.

/// Store `count` passports in a chain, oldest first, each superseding the one
/// before it. Returns their ids in that order.
async fn superseding_chain(repo: &InMemoryRepo, count: usize) -> Vec<PassportId> {
    let mut ids = Vec::with_capacity(count);
    let mut previous: Option<PassportId> = None;
    for n in 0..count {
        let mut p = draft_passport(&format!("Version {n}"));
        p.supersedes_id = previous;
        let stored = repo.create(p).await.unwrap();
        previous = Some(stored.id);
        ids.push(stored.id);
    }
    ids
}

#[tokio::test]
async fn nothing_supersedes_a_record_nothing_supersedes() {
    let repo = InMemoryRepo::default();
    let p = repo.create(draft_passport("Only")).await.unwrap();
    assert!(repo.find_superseding(p.id).await.unwrap().is_none());
    // The head walk agrees, and `None` there means "you are already holding it".
    assert!(repo.find_superseding_head(p.id).await.unwrap().is_none());
}

#[tokio::test]
async fn one_hop_finds_the_immediate_successor_and_not_the_head() {
    let repo = InMemoryRepo::default();
    let ids = superseding_chain(&repo, 3).await;

    let next = repo.find_superseding(ids[0]).await.unwrap().expect("v1");
    assert_eq!(
        next.id, ids[1],
        "one hop must stop at the immediate successor"
    );
    assert_eq!(next.product_name, "Version 1");
}

#[tokio::test]
async fn the_head_walk_reaches_the_last_record_in_the_chain() {
    let repo = InMemoryRepo::default();
    let ids = superseding_chain(&repo, 4).await;

    let head = repo
        .find_superseding_head(ids[0])
        .await
        .unwrap()
        .expect("head");
    assert_eq!(head.id, *ids.last().unwrap());
    assert_eq!(head.product_name, "Version 3");

    // Starting anywhere in the chain reaches the same head.
    let from_middle = repo
        .find_superseding_head(ids[2])
        .await
        .unwrap()
        .expect("head");
    assert_eq!(from_middle.id, *ids.last().unwrap());
}

#[tokio::test]
async fn a_successor_is_returned_whatever_its_status() {
    // Storage describes what is stored. A caller that cannot tell "no successor"
    // from "a successor you may not see" has nothing to branch on.
    for status in [PassportStatus::Draft, PassportStatus::Suspended] {
        let repo = InMemoryRepo::default();
        let ids = superseding_chain(&repo, 2).await;
        repo.update_status(ids[1], status.clone()).await.unwrap();

        let found = repo
            .find_superseding(ids[0])
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("a {status:?} successor is still a successor"));
        assert_eq!(found.status, status);
    }
}

#[tokio::test]
async fn two_records_claiming_one_predecessor_is_refused_not_arbitrated() {
    let repo = InMemoryRepo::default();
    let original = repo.create(draft_passport("Original")).await.unwrap();
    for name in ["Claimant A", "Claimant B"] {
        let mut p = draft_passport(name);
        p.supersedes_id = Some(original.id);
        repo.create(p).await.unwrap();
    }

    let err = repo
        .find_superseding(original.id)
        .await
        .expect_err("both claims are equally entitled, so neither wins");
    let msg = err.to_string();
    assert!(msg.contains("2 records claim it"), "{msg}");
    assert!(msg.contains(&original.id.to_string()), "{msg}");
}

#[tokio::test]
async fn a_cycle_is_refused_rather_than_walked_forever() {
    let repo = InMemoryRepo::default();
    let a = repo.create(draft_passport("A")).await.unwrap();
    let mut b = draft_passport("B");
    b.supersedes_id = Some(a.id);
    let b = repo.create(b).await.unwrap();

    // Close the loop: A now claims to supersede B, which supersedes A.
    let mut a_cyclic = a.clone();
    a_cyclic.supersedes_id = Some(b.id);
    repo.update(a_cyclic).await.unwrap();

    let err = repo
        .find_superseding_head(a.id)
        .await
        .expect_err("a cycle has no head");
    assert!(err.to_string().contains("it is a cycle"), "{err}");
}

#[tokio::test]
async fn a_chain_past_the_hop_cap_fails_rather_than_truncating() {
    let repo = InMemoryRepo::default();
    let ids = superseding_chain(&repo, MAX_SUCCESSION_HOPS + 2).await;

    let err = repo
        .find_superseding_head(ids[0])
        .await
        .expect_err("handing back a non-head as the head is undetectable by the caller");
    assert!(
        err.to_string()
            .contains(&format!("longer than {MAX_SUCCESSION_HOPS} hops")),
        "{err}"
    );
}

/// A chain of **exactly** `MAX_SUCCESSION_HOPS` hops is at the cap, not past it,
/// and the contract says the error is for a chain "longer than" the cap.
///
/// The loop follows `MAX_SUCCESSION_HOPS` hops and then errored without ever
/// asking whether the record it landed on has a successor — so the longest
/// permitted chain was reported as too long, and the caller lost a head that
/// exists. One inclusive probe tells the two cases apart.
#[tokio::test]
async fn a_chain_of_exactly_the_hop_cap_resolves_to_its_head() {
    let repo = InMemoryRepo::default();
    // `count` records is `count - 1` hops, so the cap in hops needs one more.
    let ids = superseding_chain(&repo, MAX_SUCCESSION_HOPS + 1).await;

    let head = repo
        .find_superseding_head(ids[0])
        .await
        .expect("a chain at the cap is within it, not beyond it")
        .expect("the chain has a head");
    assert_eq!(
        head.id,
        *ids.last().unwrap(),
        "resolved to the wrong record at exactly the cap"
    );
}
