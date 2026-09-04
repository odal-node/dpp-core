//! The passport repository contract, exercised against an in-memory double.

use super::port::*;
use async_trait::async_trait;

use crate::error::DppError;
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
struct InMemoryRepo {
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
    async fn find_published_by_gtin(&self, _gtin: &str) -> Result<Option<Passport>, DppError> {
        Ok(None)
    }
    async fn find_by_gtin_any_status(&self, gtin: &str) -> Result<Option<Passport>, DppError> {
        // Mirrors the indexed query: match the GS1 Digital Link path
        // segment rather than a bare substring, and refuse a non-numeric
        // value so a `LIKE` metacharacter cannot widen the match.
        if gtin.is_empty() || !gtin.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(None);
        }
        let needle = format!("/01/{gtin}/");
        Ok(self
            .store
            .lock()
            .unwrap()
            .values()
            .find(|p| {
                p.qr_code_url
                    .as_deref()
                    .is_some_and(|u| u.contains(&needle))
            })
            .cloned())
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
        _status: Option<PassportStatus>,
        _q: Option<&str>,
        _facility_id: Option<&str>,
        _limit: u32,
        _offset: u32,
    ) -> Result<Vec<Passport>, DppError> {
        Ok(self.store.lock().unwrap().values().cloned().collect())
    }
    async fn count(
        &self,
        _status: Option<PassportStatus>,
        _facility_id: Option<&str>,
    ) -> Result<u64, DppError> {
        Ok(self.store.lock().unwrap().len() as u64)
    }
}

fn draft_passport(name: &str) -> Passport {
    Passport {
        product_name: name.into(),
        manufacturer: ManufacturerInfo {
            name: "Brand".into(),
            address: "Berlin, DE".into(),
            did_web_url: None,
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
                "uri": "https://id.example.com/dpp/cell",
                "publicJwsHash": "00",
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
        gtin: "09506000134352".into(),
        batch_id: Some("BATCH-1".into()),
    };
    let found = repo.find_by_identity(&identity).await.unwrap();
    assert_eq!(found.map(|p| p.id), Some(created.id));

    let no_match = ProductIdentity {
        product_group: ProductGroup::Battery,
        gtin: "00000000000000".into(),
        batch_id: None,
    };
    assert!(repo.find_by_identity(&no_match).await.unwrap().is_none());
}

#[tokio::test]
async fn any_status_gtin_lookup_still_finds_a_suspended_passport() {
    // The distinction this method exists for: a withdrawn passport must stay
    // reachable by GTIN so the scanned-code route can answer `410 Gone`
    // rather than `404`. `find_published_by_gtin` cannot express it — its
    // `None` means "unknown GTIN" and "withdrawn" at once.
    let repo = InMemoryRepo::default();
    let mut p = draft_passport("Suspended battery");
    p.qr_code_url = Some("https://id.example/01/09506000134352/21/ABC123".into());
    p.status = PassportStatus::Suspended;
    let created = repo.create(p).await.unwrap();

    let found = repo
        .find_by_gtin_any_status("09506000134352")
        .await
        .unwrap();
    assert_eq!(
        found.map(|p| p.status),
        Some(PassportStatus::Suspended),
        "a suspended passport must remain findable by GTIN, and carry its status"
    );
    assert_eq!(
        repo.find_by_gtin_any_status("09506000134352")
            .await
            .unwrap()
            .map(|p| p.id),
        Some(created.id)
    );

    // An unknown GTIN is the genuine `None` — the case 404 is for.
    assert!(
        repo.find_by_gtin_any_status("00000000000000")
            .await
            .unwrap()
            .is_none()
    );
    // A non-numeric value is refused before it can act as a LIKE pattern.
    assert!(repo.find_by_gtin_any_status("%").await.unwrap().is_none());
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
