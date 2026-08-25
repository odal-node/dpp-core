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
    passport::{Passport, PassportId},
    product_identity::ProductIdentity,
    status::PassportStatus,
};
use crate::error::DppError;

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
