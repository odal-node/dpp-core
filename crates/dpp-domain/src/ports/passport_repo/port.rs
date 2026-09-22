//! [`PassportRepository`] — the persistence boundary for passport records.

use async_trait::async_trait;

use super::protected_fields::PROTECTED_PATCH_FIELDS;
use crate::error::DppError;
use crate::{
    identifier::ProductIdentifier,
    passport::{Passport, PassportId},
    product::ProductIdentity,
    status::PassportStatus,
};

/// How far
/// [`find_superseding_head`](PassportRepository::find_superseding_head) will
/// walk before refusing to answer.
///
/// **Not derived from any legal limit** — no instrument caps how often a
/// passport may be amended, and pretending otherwise would put a number in this
/// crate that no act supports. It is a cost bound: a store that has become
/// inconsistent must not be able to turn one forward resolution into an
/// unbounded sequence of queries. Reaching it is an error rather than a
/// truncated answer, so the bound can never be mistaken for a result.
///
/// Generous against the real shape of the thing it bounds. A passport amended
/// monthly across a fifteen-year retention period reaches 180; this is past that
/// with room, and still small enough that hitting it means something is wrong.
pub const MAX_SUCCESSION_HOPS: usize = 256;

/// Port trait for all DPP persistence operations.
///
/// **No physical delete method is defined by design.** Read against the
/// verbatim OJ text of Regulation (EU) 2024/1781: **Art. 9(2)(i)** requires the
/// delegated act to specify the period a passport must remain available, which
/// "shall correspond to at least the expected lifetime of a specific product",
/// and **Art. 11(e)** makes that availability an essential requirement holding
/// even after insolvency, liquidation or cessation of activity. In practice
/// that is a retention period of typically 10–15 years. Passports transition through statuses
/// (Draft → Published → Suspended → Retired) but are never physically removed.
/// Any cleanup job or admin tooling MUST check `retention_locked` before
/// removing a record from the database.
#[async_trait]
pub trait PassportRepository: Send + Sync {
    async fn create(&self, passport: Passport) -> Result<Passport, DppError>;

    async fn find_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError>;

    /// Fetch a passport by ID — for public resolver use.
    /// Returns `None` if not found or not in Published state.
    async fn find_published_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError>;

    /// Every passport whose product identifier is `identifier`, narrowed by
    /// `batch_id` and `serial_number` when the caller has them.
    ///
    /// Replaces the two by-GTIN lookups this port used to carry. Three things
    /// changed and each was a defect:
    ///
    /// 🚨 **An identifier does not name one passport.** `Granularity` admits
    /// `Model`, `Batch` and `Item`, so a single GTIN can have a model passport
    /// and one per production run and one per unit. Measured: two published
    /// passports sharing `09506000134352` across two batches. The old methods
    /// were documented as returning *"the first"*, with no ordering defined —
    /// one arbitrary row of N. Hence `Vec`, and hence the two narrowing
    /// arguments: `(None, None)` is the model-level record, a batch narrows to
    /// one run, a serial to one unit. That is the shape of the GS1 Digital Link
    /// the caller is usually answering — `/01/{gtin}`, `/01/{gtin}/10/{lot}`,
    /// `/01/{gtin}/21/{serial}`.
    ///
    /// 🚨 **Typed, not `&str`.** A bare string invites taking a value out of
    /// one namespace and looking it up in another because both are strings,
    /// which is how a GTIN came to be scraped out of a carrier URI. The route
    /// being served already determines the scheme, so constructing a
    /// [`ProductIdentifier`] at that boundary costs the caller nothing and
    /// makes the knowledge explicit.
    ///
    /// 🚨 **No status filter, and no `Option`.** `find_published_by_gtin`
    /// folded a publication-policy decision into a lookup, so `None` meant both
    /// "no such product" and "it exists but is not published" — and a public
    /// route needs to tell those apart to serve `410 Gone` rather than `404`,
    /// which is the recall signal a consumer scanning a product has to see.
    /// Storage describes what is stored; which statuses an audience may see is
    /// domain policy and is applied by the caller.
    ///
    /// Returning `Option` here would also encode an invariant this tier cannot
    /// enforce. At most one matching passport should be `Published` at a time —
    /// a superseded predecessor moves to `Superseded` — but nothing in a pure
    /// core can hold storage to that, and a signature that silently picks one
    /// of two is the defect being removed, not a smaller version of it.
    ///
    /// # Ordering
    ///
    /// Unspecified. A caller wanting the current record filters on `status`
    /// rather than taking the first element.
    ///
    /// Default implementation is an unindexed `list()` scan — correctness only,
    /// suitable for tests and small in-memory stores. 🚨 A real implementation
    /// **must index the identifier**, which is not free: it lives inside
    /// `productGroupData`, so it needs a generated column or an expression
    /// index rather than a plain one.
    async fn find_by_identifier(
        &self,
        identifier: &ProductIdentifier,
        batch_id: Option<&str>,
        serial_number: Option<&str>,
    ) -> Result<Vec<Passport>, DppError> {
        // `None` is every status, by the same reading as `facility_id`
        // below. One pass: a per-status loop would return the whole store once
        // per status against any implementation whose filter is a no-op, which
        // is a failure mode a default implementation should not depend on
        // avoiding.
        Ok(self
            .list(None, None, None, u32::MAX, 0)
            .await?
            .into_iter()
            .filter(|p| {
                p.product_group_data
                    .as_ref()
                    .and_then(crate::ProductGroupData::product_identifier)
                    .is_some_and(|id| id == identifier)
                    && p.batch_id.as_deref() == batch_id
                    && p.serial_number.as_deref() == serial_number
            })
            .collect())
    }

    /// Fetch a passport by ID regardless of status.
    /// Used by public endpoints to distinguish between 404 and 410 (suspended).
    async fn find_by_id_any_status(&self, id: PassportId) -> Result<Option<Passport>, DppError>;

    /// Find a passport by exact compound identity — product group, unique product
    /// identifier and batch —
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

    /// Find the record that supersedes `id` — the reverse of `supersedes_id`.
    ///
    /// # Why the edge can only be read in this direction by querying
    ///
    /// A superseded passport keeps its identifier forever, and where that
    /// identifier is the product's printed data carrier, a reader holding the
    /// physical product arrives at the **predecessor** after an amendment. The
    /// only correct destinations from there are the successor or an explicit
    /// statement that the record was replaced, and both need this lookup.
    ///
    /// The forward pointer cannot be stored on the predecessor. `supersedes_id`
    /// points backwards by design, and writing a successor pointer onto the
    /// predecessor would mutate a published record: it is in
    /// [`PROTECTED_PATCH_FIELDS`], and the body is frozen at publish and covered
    /// by the signature over it, so the write is either refused or it invalidates
    /// the proof. The edge therefore exists only as the successor's backward
    /// pointer, and reading it usefully is a query rather than a field access.
    ///
    /// # What it does not do
    ///
    /// **No status filter.** A successor that is itself superseded, or one still
    /// in `Draft`, is returned like any other. Storage describes what is stored;
    /// which statuses a given audience may see is domain policy, and folding it
    /// in here would repeat the mistake the removed `find_published_by_gtin`
    /// made — a caller that cannot tell "no successor" from "a successor it is
    /// not allowed to see" has nothing to branch on. See
    /// [`find_by_identifier`](PassportRepository::find_by_identifier), which
    /// now holds that reasoning.
    ///
    /// **One hop only.** A record amended more than once has a chain, and a
    /// reader usually wants its head; that is
    /// [`find_superseding_head`](PassportRepository::find_superseding_head),
    /// which is written in terms of this and needs no separate implementation.
    ///
    /// # Errors
    ///
    /// [`DppError::SuccessionUnresolvable`] where more than one record claims
    /// `id` as its predecessor. That should not happen, and saying which row
    /// wins would be worse than refusing: both are equally entitled to the
    /// claim, so any choice is arbitrary and the caller cannot tell an arbitrary
    /// answer from a real one.
    ///
    /// # Default implementation
    ///
    /// An unindexed `list()` scan — correctness only, suitable for tests and
    /// small in-memory stores, and the same arrangement
    /// [`find_by_identity`](PassportRepository::find_by_identity) uses. The
    /// natural index is on `supersedes_id`, so a real store should override this
    /// with a single indexed read.
    ///
    /// [`PROTECTED_PATCH_FIELDS`]: super::PROTECTED_PATCH_FIELDS
    async fn find_superseding(&self, id: PassportId) -> Result<Option<Passport>, DppError> {
        let claimants: Vec<Passport> = self
            .list(None, None, None, u32::MAX, 0)
            .await?
            .into_iter()
            .filter(|p| p.supersedes_id == Some(id))
            .collect();

        match claimants.len() {
            0 => Ok(None),
            1 => Ok(claimants.into_iter().next()),
            n => Err(DppError::SuccessionUnresolvable {
                id: id.to_string(),
                reason: format!("{n} records claim it as their predecessor"),
            }),
        }
    }

    /// Walk forward to the head of the amendment chain starting at `id`.
    ///
    /// `Ok(None)` means nothing supersedes `id` — it *is* the head, and the
    /// caller already holds it. `Ok(Some(p))` is the last record in the chain,
    /// the one nothing supersedes in turn.
    ///
    /// # Why this is here and not left to callers
    ///
    /// Every caller resolving a printed carrier wants the head, so every caller
    /// would write this loop, and the two ways to get it wrong are not obvious
    /// until a store is already inconsistent: an unbounded walk turns one lookup
    /// into a query storm, and a walk with no visited set never terminates on a
    /// cycle. Both are cheap here and cost nothing to implementors, since this
    /// is written in terms of
    /// [`find_superseding`](PassportRepository::find_superseding) and inherits
    /// whatever indexed read that gets.
    ///
    /// # Errors
    ///
    /// [`DppError::SuccessionUnresolvable`] on a cycle, on a chain longer than
    /// [`MAX_SUCCESSION_HOPS`], and on whatever `find_superseding` refuses. It
    /// **fails rather than truncating**: handing back the furthest record
    /// reached would be handing back a non-head as though it were the head,
    /// which is the one answer a caller cannot detect as wrong.
    async fn find_superseding_head(&self, id: PassportId) -> Result<Option<Passport>, DppError> {
        let mut seen = std::collections::HashSet::from([id]);
        let mut head: Option<Passport> = None;
        let mut cursor = id;

        for _ in 0..MAX_SUCCESSION_HOPS {
            let Some(next) = self.find_superseding(cursor).await? else {
                return Ok(head);
            };
            if !seen.insert(next.id) {
                return Err(DppError::SuccessionUnresolvable {
                    id: id.to_string(),
                    reason: format!("the chain returns to passport {} — it is a cycle", next.id),
                });
            }
            cursor = next.id;
            head = Some(next);
        }

        // 🚨 The loop has followed exactly `MAX_SUCCESSION_HOPS` hops, which is
        // the cap — not past it. Erroring here refused a chain of *exactly* the
        // permitted length, because the loop never got to ask whether the record
        // it landed on has a successor. The contract above says the error is for
        // a chain "longer than" the cap, and one probe is what tells the two
        // apart.
        if self.find_superseding(cursor).await?.is_none() {
            return Ok(head);
        }

        Err(DppError::SuccessionUnresolvable {
            id: id.to_string(),
            reason: format!("the chain is longer than {MAX_SUCCESSION_HOPS} hops"),
        })
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
    /// The lineage edges (`derivedFrom`, `componentRefs`) are protected
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
