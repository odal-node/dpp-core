//! `PassportRepository` — port for all DPP persistence operations.
//!
//! No physical delete is exposed by design: ESPR retention obligations prohibit
//! removing published passports for the applicable retention period (typically
//! 10–15 years per sector delegated act).

use async_trait::async_trait;

use crate::domain::{
    error::DppError,
    passport::{Passport, PassportId},
    product_identity::ProductIdentity,
    status::PassportStatus,
};

/// Fields governed by the state machine, the retention lock, the publish/seal
/// pipeline, or record identity — none of which is a user-editable content
/// field. `patch_fields` rejects any delta touching one of these so it cannot
/// be used to bypass `transition_to`/`update_status` (e.g. flip
/// `retentionLocked` back to `false` or forge a `jwsSignature`). Serialized
/// (camelCase) field names, matching the `Passport` JSON representation.
const PROTECTED_PATCH_FIELDS: [&str; 12] = [
    "id",
    "status",
    "retentionLocked",
    "retentionUntil",
    "jwsSignature",
    "publicJwsSignature",
    "seal",
    "version",
    "publishedAt",
    "createdAt",
    "supersedesId",
    "schemaVersion",
];

/// Port trait for all DPP persistence operations.
///
/// **No physical delete method is defined by design.** EU ESPR Article 9 and
/// sector delegated acts require published passports to remain publicly
/// accessible for the product's expected lifetime plus a defined retention
/// period (typically 10–15 years). Passports transition through statuses
/// (Draft → Published → Suspended → Archived) but are never physically removed.
/// Any cleanup job or admin tooling MUST check `retention_locked` before
/// removing a record from the database.
#[async_trait]
pub trait PassportRepository: Send + Sync {
    async fn create(&self, passport: Passport) -> Result<Passport, DppError>;

    async fn find_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError>;

    /// Fetch a passport by ID — for public resolver use.
    /// Returns `None` if not found or not in Published state.
    async fn find_published_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError>;

    /// Find the first published passport whose GS1 Digital Link QR URL contains
    /// the given 14-digit GTIN. Used by the `GET /01/{gtin}` resolver route.
    async fn find_published_by_gtin(&self, gtin: &str) -> Result<Option<Passport>, DppError>;

    /// Fetch a passport by ID regardless of status.
    /// Used by public endpoints to distinguish between 404 and 410 (suspended).
    async fn find_by_id_any_status(&self, id: PassportId) -> Result<Option<Passport>, DppError>;

    /// Find a passport by exact compound identity — sector, GTIN, and batch —
    /// across `Draft` and `Published`. Used by the import delta-matcher to
    /// classify a row as create/update_draft/conflict_published before any
    /// write. Returns `None` on no match; `batch_id: None` matches only
    /// passports with no batch set.
    ///
    /// Default implementation is an unindexed `list()` scan — correctness
    /// only, suitable for tests and small in-memory stores. `PgPassportRepo`
    /// overrides this with a real indexed query.
    async fn find_by_identity(
        &self,
        identity: &ProductIdentity,
    ) -> Result<Option<Passport>, DppError> {
        let drafts = self
            .list(Some(PassportStatus::Draft), None, None, u32::MAX, 0)
            .await?;
        let published = self
            .list(Some(PassportStatus::Published), None, None, u32::MAX, 0)
            .await?;
        Ok(drafts
            .into_iter()
            .chain(published)
            .find(|p| ProductIdentity::from_passport(p).as_ref() == Some(identity)))
    }

    async fn update(&self, passport: Passport) -> Result<Passport, DppError>;

    /// Merge a JSON delta into an existing passport, touching only the
    /// specified fields. Safer than `update()` for user-initiated field
    /// edits: concurrent patches to different fields do not clobber each
    /// other. The default implementation falls back to the read-modify-write
    /// pattern — implementations should override with a targeted MERGE
    /// statement for real concurrent-write safety.
    ///
    /// A delta that tries to set any `PROTECTED_PATCH_FIELDS` key (status,
    /// retention lock, signatures, seal, identity, …) is rejected with
    /// [`DppError::Validation`]: those transitions belong to the state machine
    /// (`transition_to`/`update_status`) and the publish pipeline, never to a
    /// free-form field patch.
    async fn patch_fields(
        &self,
        id: PassportId,
        delta: serde_json::Value,
    ) -> Result<Passport, DppError> {
        if let Some(obj) = delta.as_object() {
            let mut forbidden: Vec<&str> = PROTECTED_PATCH_FIELDS
                .iter()
                .copied()
                .filter(|k| obj.contains_key(*k))
                .collect();
            if !forbidden.is_empty() {
                forbidden.sort_unstable();
                return Err(DppError::Validation(
                    format!(
                        "patch_fields cannot modify protected field(s): {}",
                        forbidden.join(", ")
                    )
                    .into(),
                ));
            }
        }

        let Some(mut passport) = self.find_by_id(id).await? else {
            return Err(DppError::NotFound(id.to_string()));
        };
        let mut p_val = serde_json::to_value(&passport)
            .map_err(|e| DppError::Internal(format!("serialize: {e}")))?;
        if let (serde_json::Value::Object(pm), serde_json::Value::Object(dm)) = (&mut p_val, delta)
        {
            pm.extend(dm);
        }
        passport = serde_json::from_value(p_val)
            .map_err(|e| DppError::Internal(format!("deserialize: {e}")))?;
        self.update(passport).await
    }

    async fn update_status(
        &self,
        id: PassportId,
        status: PassportStatus,
    ) -> Result<Passport, DppError>;

    /// `facility_id` filters to passports stamped with that exact facility
    /// identifier (ESPR Annex III; ADR-006 grouping, not isolation — see
    /// `Passport::facility`). `None` returns passports for every facility.
    async fn list(
        &self,
        status: Option<PassportStatus>,
        q: Option<&str>,
        facility_id: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Passport>, DppError>;

    /// Total number of passports (ignoring pagination).
    /// Optional `status` and `facility_id` filters; `None` counts every match.
    async fn count(
        &self,
        status: Option<PassportStatus>,
        facility_id: Option<&str>,
    ) -> Result<u64, DppError>;

    // ─── Batch operations ────────────────────────────────────────────────

    /// Create multiple passports in a single batch operation.
    ///
    /// Suitable for bulk manufacturer uploads where thousands of DPPs are
    /// ingested at once. Platform implementations should override this with
    /// optimized concurrent or pipelined persistence (e.g. multi-row INSERT,
    /// connection pooling, or chunked parallelism).
    ///
    /// Returns one `Result` per input passport, in the same order.
    /// Partial success is allowed — some items may succeed while others fail.
    ///
    /// The default implementation falls back to sequential `create` calls.
    async fn create_batch(&self, passports: Vec<Passport>) -> Vec<Result<Passport, DppError>> {
        let mut results = Vec::with_capacity(passports.len());
        for passport in passports {
            results.push(self.create(passport).await);
        }
        results
    }

    /// Update multiple passports in a single batch operation.
    ///
    /// Same semantics as `create_batch` — returns per-item results,
    /// partial success is allowed.
    ///
    /// The default implementation falls back to sequential `update` calls.
    async fn update_batch(&self, passports: Vec<Passport>) -> Vec<Result<Passport, DppError>> {
        let mut results = Vec::with_capacity(passports.len());
        for passport in passports {
            results.push(self.update(passport).await);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::passport::ManufacturerInfo;
    use crate::domain::sector::Sector;
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
        async fn find_by_id_any_status(
            &self,
            id: PassportId,
        ) -> Result<Option<Passport>, DppError> {
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
            id: PassportId::new(),
            batch_id: None,
            product_name: name.into(),
            sector: Sector::Textile,
            product_category: None,
            manufacturer: ManufacturerInfo {
                name: "Brand".into(),
                address: "Berlin, DE".into(),
                did_web_url: None,
            },
            materials: vec![],
            co2e_per_unit: None,
            repairability_score: None,
            compliance_result: None,
            lint_result: None,
            sector_data: None,
            status: PassportStatus::Draft,
            qr_code_url: None,
            jws_signature: None,
            public_jws_signature: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            published_at: None,
            schema_version: "1.1.0".into(),
            retention_locked: false,
            version: 1,
            supersedes_id: None,
            parent_passport_ref: None,
            component_refs: Vec::new(),
            retention_until: None,
            product_id: None,
            operator_identifier: None,
            facility: None,
            seal: None,
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
        use crate::domain::gtin::Gtin;
        use crate::domain::sector::{BatteryChemistry, BatteryData, SectorData};

        let repo = InMemoryRepo::default();
        let mut p = draft_passport("Battery A");
        p.sector = Sector::Battery;
        p.sector_data = Some(SectorData::Battery(BatteryData {
            gtin: Gtin::parse("09506000134352").unwrap(),
            battery_chemistry: BatteryChemistry::Lfp,
            nominal_voltage_v: 3.2,
            nominal_capacity_ah: 100.0,
            expected_lifetime_cycles: 3000,
            co2e_per_unit_kg: 85.4,
            recycled_content_cobalt_pct: None,
            recycled_content_lithium_pct: None,
            recycled_content_nickel_pct: None,
            state_of_health_pct: None,
            rated_capacity_kwh: None,
            carbon_footprint_class: None,
            due_diligence_url: None,
            cathode_material: None,
            anode_material: None,
            electrolyte_material: None,
            critical_raw_materials: None,
            disassembly_instructions_url: None,
            soh_methodology: None,
            operating_temp_min_c: None,
            operating_temp_max_c: None,
            rated_energy_wh: None,
            recycled_content_lead_pct: None,
            battery_weight_kg: None,
            battery_type: None,
            round_trip_efficiency_pct: None,
            internal_resistance_mohm: None,
            manufacturing_date: None,
            manufacturing_place: None,
            battery_model_id: None,
            battery_passport_number: None,
        }));
        p.batch_id = Some("BATCH-1".into());
        let created = repo.create(p).await.unwrap();

        let identity = ProductIdentity {
            sector: Sector::Battery,
            gtin: "09506000134352".into(),
            batch_id: Some("BATCH-1".into()),
        };
        let found = repo.find_by_identity(&identity).await.unwrap();
        assert_eq!(found.map(|p| p.id), Some(created.id));

        let no_match = ProductIdentity {
            sector: Sector::Battery,
            gtin: "00000000000000".into(),
            batch_id: None,
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
}
