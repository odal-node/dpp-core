//! `PassportRepository` — port for all DPP persistence operations.
//!
//! No physical delete is exposed by design: ESPR retention obligations prohibit
//! removing published passports for the applicable retention period (typically
//! 10–15 years per product group delegated act).
//!
//! # Art. 78(d) — what an implementor may do with this data
//!
//! Regulation (EU) 2023/1542 Art. 78(d): where passport data is stored or
//! otherwise processed by operators authorised to act on behalf of the
//! responsible economic operator, those operators *"shall not be allowed to
//! sell, re-use or process such data, in whole or in part, beyond what is
//! necessary for the provision of the relevant storing or processing
//! services"*.
//!
//! This port is the primary surface that constraint applies to, and [`list`] and
//! [`count`] are the parts of it that see more than one passport at a time. An
//! implementation backing a **hosted** node is a processor in the Art. 78(d)
//! sense, and may use those methods only to serve the operator's own requests —
//! not to derive cross-customer benchmarks, train models, or produce analytics
//! the operator did not ask for.
//!
//! The prohibition is on the *processor*, not the operator: an operator
//! analysing its own passports is doing nothing this article restricts.
//!
//! [`list`]: PassportRepository::list
//! [`count`]: PassportRepository::count

use async_trait::async_trait;

use crate::domain::{
    error::DppError,
    passport::{Passport, PassportId},
    product_identity::ProductIdentity,
    status::PassportStatus,
};

/// Fields governed by the state machine, the retention lock, the publish/seal
/// pipeline, record identity, or a dedicated transition method — none of
/// which is a user-editable content field. `patch_fields` rejects any delta
/// touching one of these so it cannot be used to bypass `transition_to`/
/// `update_status` (e.g. flip `retentionLocked` back to `false` or forge a
/// `jwsSignature`), or `RegistrySyncPort::notify_transfer` (change
/// `operatorIdentifier` without going through transfer-of-responsibility).
/// `facility` is likewise excluded: it is a point-in-time snapshot copied at
/// create time by design, not a field any flow updates in place. Serialized
/// (camelCase) field names, matching the `Passport` JSON representation.
///
/// # Why this is `pub`
///
/// It is part of this trait's documented behaviour — the `patch_fields` doc
/// below tells a caller which keys are refused — and a backend that overrides
/// the default implementation still owes callers that contract. Keeping it
/// private meant an implementor had no way to honour it except by retyping the
/// list, and a retyped list drifts: the PostgreSQL backend's copy fell three
/// entries short (`operatorIdentifier`, `facility`, `parentPassportRef`), which
/// on the only backend that ships made those fields writable through a
/// user-facing field patch and carried them into the signed publish payload.
///
/// An implementation that must differ should derive its list *from this value*
/// — adding or removing named entries with the reason stated — never restate it.
///
/// A slice rather than a fixed-size array on purpose: the length is not part of
/// the type, so adding an entry here does not break a consumer that annotated
/// one.
pub const PROTECTED_PATCH_FIELDS: &[&str] = &[
    "id",
    "status",
    "retentionLocked",
    "retentionUntil",
    "jwsSignature",
    "publicJwsSignature",
    // The fourth proof field, alongside the two signatures and the seal. It was
    // the only one of the four missing here while the PostgreSQL backend's copy
    // protected it — so this list, the one every other implementation inherits,
    // was the weaker of the two.
    "disclosureSignatures",
    "seal",
    "version",
    "publishedAt",
    "createdAt",
    "supersedesId",
    "schemaVersion",
    "operatorIdentifier",
    "facility",
    "parentPassportRef",
    "componentRefs",
    // The applicable law at placing on the market does not change. A
    // mis-recorded set is corrected by superseding the passport, never by
    // patching a published record's legal basis.
    "applicableInstruments",
];

/// Port trait for all DPP persistence operations.
///
/// **No physical delete method is defined by design.** Read against the
/// verbatim OJ text of Regulation (EU) 2024/1781: **Art. 9(2)(i)** requires the
/// delegated act to specify the period a passport must remain available, which
/// "shall correspond to at least the expected lifetime of a specific product",
/// and **Art. 11(e)** makes that availability an essential requirement holding
/// even after insolvency, liquidation or cessation of activity. In practice
/// that is a retention period of typically 10–15 years. Passports transition through statuses
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
    ///
    /// Folds a publication-policy decision into a lookup: `None` means both "no
    /// such GTIN" and "that GTIN resolves to a passport that is not published",
    /// and a caller has nothing to branch on. A **public route must not use
    /// this** — it cannot then tell a withdrawn passport from an unregistered
    /// one, and so cannot serve `410 Gone`, which is the recall signal a
    /// consumer scanning a product needs to see. Use
    /// [`find_by_gtin_any_status`](PassportRepository::find_by_gtin_any_status)
    /// there and branch on `status`, the way the by-id route already does.
    async fn find_published_by_gtin(&self, gtin: &str) -> Result<Option<Passport>, DppError>;

    /// Find the first passport whose GS1 Digital Link QR URL contains the given
    /// 14-digit GTIN, regardless of status.
    ///
    /// The by-GTIN counterpart of
    /// [`find_by_id_any_status`](PassportRepository::find_by_id_any_status),
    /// and it exists for the same reason: a public endpoint has to distinguish
    /// 404 from 410 (suspended), and it can only do that if the lookup hands
    /// back the passport and leaves the lifecycle decision to whoever is
    /// answering the request. Storage describes what is stored; which statuses
    /// are publicly visible is domain policy and does not belong here.
    async fn find_by_gtin_any_status(&self, gtin: &str) -> Result<Option<Passport>, DppError>;

    /// Fetch a passport by ID regardless of status.
    /// Used by public endpoints to distinguish between 404 and 410 (suspended).
    async fn find_by_id_any_status(&self, id: PassportId) -> Result<Option<Passport>, DppError>;

    /// Find a passport by exact compound identity — product group, GTIN, and batch —
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
    /// retention lock, signatures, seal, identity, operator, facility,
    /// lineage, …) is rejected with [`DppError::Validation`]: those
    /// transitions belong to the state machine (`transition_to`/
    /// `update_status`), the publish pipeline, or a dedicated transfer method,
    /// never to a free-form field patch.
    ///
    /// The lineage edges (`parentPassportRef`, `componentRefs`) are protected
    /// because they are create-time by construction and sit inside the signed
    /// public view: a second-life passport is a *new* record issued alongside
    /// its predecessors, and changing a published bill of materials is a new
    /// passport version (`supersedesId`), not an in-place edit. Patching either
    /// would leave the served body no longer verifying against its own
    /// signature. See `docs/architecture/PRODUCT-LINEAGE.md`.
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
    /// identifier (ESPR Annex III). It is a grouping filter, not an isolation
    /// boundary — see `Passport::facility`. `None` returns passports for every
    /// facility.
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
    use crate::domain::product_group::ProductGroup;
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
                "parentPassportRef": {
                    "uri": "https://id.example.com/dpp/other",
                    "publicJwsHash": "00",
                }
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
        assert!(stored.parent_passport_ref.is_none());
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
        use crate::domain::product_group::ProductGroupData;

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
}

#[cfg(test)]
mod protected_patch_fields_tests {
    use super::PROTECTED_PATCH_FIELDS;
    use crate::domain::passport::{FacilitySnapshot, PassportId, PassportRef};
    use crate::domain::seal::{SealFormat, SealedEnvelope};
    use crate::test_support::sample_passport;
    use chrono::Utc;

    /// Every protected key must be a key `Passport` actually serialises to.
    ///
    /// The list is strings, and nothing else ties them to the struct. A renamed
    /// field — or a typo — leaves an entry protecting a key that no longer
    /// exists, and the field it was meant to protect silently becomes patchable.
    /// That failure is invisible: the guard still runs, still finds nothing, and
    /// still reports success.
    ///
    /// The instance below sets every `skip_serializing_if` field, because a
    /// field that is `None` does not appear in the JSON at all and would look
    /// exactly like a stale entry.
    #[test]
    fn every_protected_key_is_a_real_passport_field() {
        let now = Utc::now();
        let reference = PassportRef {
            uri: "https://id.example/dpp/1".to_owned(),
            public_jws_hash: "0".repeat(64),
        };
        let mut passport = sample_passport();
        passport.public_jws_signature = Some("eyJ..a".to_owned());
        passport
            .disclosure_signatures
            .insert("public+restricted".to_owned(), "eyJ..b".to_owned());
        passport.retention_until = Some(now);
        passport.supersedes_id = Some(PassportId::new());
        passport.parent_passport_ref = Some(reference.clone());
        passport.component_refs = vec![reference];
        passport.operator_identifier = Some("DE123456789".to_owned());
        passport.facility = Some(FacilitySnapshot {
            scheme: "gln".to_owned(),
            value: "4012345000009".to_owned(),
            name: "Werk Nord".to_owned(),
            country: "DE".to_owned(),
            address: None,
        });
        passport.seal = Some(SealedEnvelope {
            format: SealFormat::Cades,
            seal_value: "MIIB".to_owned(),
            signing_cert_ref: None,
            sealed_at: now,
            placeholder: true,
        });

        let json = serde_json::to_value(&passport).expect("passport serialises");
        let obj = json.as_object().expect("passport is a JSON object");

        let stale: Vec<&str> = PROTECTED_PATCH_FIELDS
            .iter()
            .copied()
            .filter(|k| !obj.contains_key(*k))
            .collect();

        assert!(
            stale.is_empty(),
            "PROTECTED_PATCH_FIELDS names keys `Passport` does not serialise to: {stale:?}\n\
             Either the field was renamed and this list was not, or the instance above \
             does not populate it. Both mean the field is unprotected."
        );
    }

    /// No duplicates — a repeated entry is a sign the list was edited by hand in
    /// two places, which is the failure mode this list exists to end.
    #[test]
    fn protected_keys_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for key in PROTECTED_PATCH_FIELDS {
            assert!(seen.insert(*key), "duplicate protected key: {key}");
        }
    }
}
