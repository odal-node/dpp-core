//! The [`Passport`] aggregate root.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    DerivationRef, FacilitySnapshot, ManufacturerInfo, MaterialEntry, PassportId, PassportRef,
};
use crate::catalog::Granularity;
use crate::compliance::ComplianceResult;
use crate::instrument::InstrumentRef;
use crate::seal::SealedEnvelope;
use crate::{
    lint::LintResult,
    product_group::{CarbonFootprint, ProductGroup, ProductGroupData, RepairabilityScore},
    status::PassportStatus,
};

/// The canonical Digital Product Passport record as defined by EU ESPR.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Passport {
    pub id: PassportId,
    /// Optional batch or lot identifier.
    pub batch_id: Option<String>,
    pub product_name: String,
    /// EU ESPR product group — the delegated-act bucket that selects the applicable
    /// schema and plugin. (Replaces the former misnamed `product_category`
    /// field, which actually held a product group.)
    pub product_group: ProductGroup,
    /// The legal instruments recorded as applicable to this product, fixed when
    /// it was placed on the market.
    ///
    /// **Recorded, never recomputed** — see [`InstrumentRef`]. ESPR Art. 5(7)
    /// lets acts overlap with no precedence rule between them, so this is a set
    /// and the governing law is the union of its members' requirements; and
    /// because a horizontal act can reach a product whose product group no
    /// catalog models, the set cannot be derived from
    /// [`Self::product_group`] at all. That is why it is stored rather than
    /// looked up, and why it is protected from patching: re-deriving it would
    /// silently drop every entry a human had to supply.
    ///
    /// Empty on a record issued before this field existed, which is a statement
    /// that nothing was recorded — not that nothing applies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applicable_instruments: Vec<InstrumentRef>,
    /// The level this passport describes: one model, one batch, or one item.
    ///
    /// ESPR Art. 9(2)(d) makes this a **delegated-act decision**, so it is a
    /// property of the applicable law rather than an implementer's choice, and
    /// `None` is the honest answer while no adopted act has fixed a level —
    /// which is every product group today. Do not default it: the EU registry
    /// registers batteries at item level, but that is the registry's operational
    /// position and not a level any act has set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granularity: Option<Granularity>,
    pub manufacturer: ManufacturerInfo,
    pub materials: Vec<MaterialEntry>,
    /// CO₂ equivalent per unit — manufacturer-supplied or engine-calculated.
    pub co2e_per_unit: Option<CarbonFootprint>,
    /// Repairability score (non-regulatory heuristic — not EN 45554 / EU 2023/1669).
    pub repairability_score: Option<RepairabilityScore>,
    /// The computed compliance determination — status, metrics, binding
    /// `violations` + advisory `warnings`, and (when a calculation ran) a
    /// receipt. Attached by the engine's `apply_compliance` at create/update.
    /// Part of the signed payload and immutable after retention lock. `None`
    /// until a determination is computed (e.g. a product group with no plugin loaded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compliance_result: Option<ComplianceResult>,
    /// Non-binding plausibility findings from the `dpp-rules` lint pack —
    /// arithmetic and physical-plausibility checks distinct from binding
    /// compliance rules. Never gates publish and may be recomputed at any
    /// time after publish (a lint re-check), unlike `compliance_result` —
    /// see the vault's `POST /dpp/{id}/lint` endpoint. `None` until a lint
    /// pass has run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint_result: Option<LintResult>,
    /// Typed, product group-specific DPP data (EU Battery Regulation, Textile DPP, etc.).
    ///
    /// `None` for passports where product group-specific data has not yet been supplied.
    /// Set this field when publishing to ensure regulatory compliance validation.
    pub product_group_data: Option<ProductGroupData>,
    pub status: PassportStatus,
    /// The publicly accessible QR code URL for this passport.
    pub qr_code_url: Option<String>,
    /// Compact JWS signature over the **full** canonical passport payload
    /// (`Disclosure::Conformity` — for authenticated, full-passport verification).
    pub jws_signature: Option<String>,
    /// Compact JWS signature over the **public (redacted) view** of this passport
    /// (`Disclosure::Public`). Lets anyone verify the public passport independently — the
    /// resolver checks this on the unauthenticated `/public/dpp/{id}` route.
    /// Set at publish time; `None` for drafts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_jws_signature: Option<String>,
    /// Compact JWS signatures over the **non-public** redacted views, keyed by
    /// [`disclosure_key`](crate::disclosure::disclosure_key) — e.g.
    /// `public+restricted+individual`.
    ///
    /// Every audience that receives more than the public view needs a proof over
    /// *its* view: `public_jws_signature` covers only the public payload and
    /// `jws_signature` only the full one, so a reader given a filtered body and
    /// either of those holds a signature that cannot verify against the bytes it
    /// received. A repairer or recycler making a safety or resale call on the
    /// data is precisely the caller who must be able to check it.
    ///
    /// **Keyed by disclosure set, never by audience name.** ESPR's actor
    /// vocabulary is not battery Art. 77(2)'s three audiences, and the delegated
    /// act mapping actors to data is unadopted. An artefact named for the data it
    /// covers survives that mapping arriving; one named `"legitimateInterest"`
    /// would need every passport re-signed.
    ///
    /// A `BTreeMap` so serialisation is key-ordered and the signed bytes are
    /// reproducible. Frozen at publish alongside its two siblings, and empty for
    /// drafts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub disclosure_signatures: BTreeMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    /// The date this product was placed on the EU market — the regulated
    /// triggering event that fixes **which law governs it**.
    ///
    /// Distinct from every other date on this struct, all of which describe
    /// what *this record* did: `created_at`, `updated_at` and `published_at`
    /// are passport lifecycle, and none of them selects a rule. Staged EU
    /// obligations attach at placing on the market and do not move afterwards —
    /// a product lawfully placed on the market in 2030 does not acquire a 2031
    /// minimum by being reassessed in 2033 — so a determination made against
    /// today's date is wrong for every product not placed on the market today.
    ///
    /// Envelope-level rather than per-product group because the triggering event is
    /// not product group-specific: ESPR attaches its duties at placing on the market
    /// for every product group, as do Regulation (EU) 2023/1542 Art. 7, 8 and
    /// 10 for batteries. It lived only on `BatteryData` before, which made the
    /// governing law underivable for the other eleven product groups.
    ///
    /// `None` means the date was not declared, which is **not** a licence to
    /// substitute the current date. A determination that depends on it has no
    /// answer, and saying so is the answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed_on_market_date: Option<NaiveDate>,
    /// Semantic version of the *product group* schema used to validate this record.
    ///
    /// Scoped to `product_group_data` only — there is no equivalent version for the
    /// envelope fields on this struct. [`Passport::from_stored`] uses this to
    /// decide whether `product_group_data` needs upcasting through a lens before
    /// this record can be re-read. Envelope fields have no such escape hatch
    /// and never will: a lens transforms one product group's sub-object, but an
    /// envelope field is shared by every product group's stored documents, so a
    /// non-additive envelope change would need a transform over the whole
    /// document — one mistake there corrupts every product group at once, not one.
    /// The envelope's rule is therefore additive-only: `Option<T>` +
    /// `#[serde(default)]`, or a rename that keeps accepting the old key, never
    /// a bare requirement added to an existing field.
    ///
    /// **That rule has now been broken twice, deliberately both times, and
    /// recording it is more useful than restating the rule as absolute.**
    /// `sector` → `productGroup` and `parentPassportRef` → `derivedFrom` each
    /// renamed an envelope key without keeping the old one readable. They failed
    /// differently, and the second was the more dangerous: `product_group` is
    /// required, so a pre-rename document refuses to deserialize outright,
    /// whereas this struct sets no `deny_unknown_fields` and `derived_from`
    /// defaults — so a document carrying `parentPassportRef` loaded
    /// *successfully*, silently arriving with no lineage edge at all.
    ///
    /// That silence is now closed: [`REMOVED_ENVELOPE_KEYS`] records the old key
    /// and [`Passport::from_stored`] refuses any document carrying one, so both
    /// renames fail loudly on the supported read path.
    ///
    /// Both were taken while this project has no published passports to strand.
    /// **That licence ends the moment one exists.** After that, an envelope
    /// rename has to carry the old key or a one-time document rewrite in the
    /// publish pipeline. Refusing to read a document is the right failure, but
    /// it is still a failure: a rewritten document no longer verifies against a
    /// signature that covers the old key names, so there is no version of this
    /// that a real passport survives without a migration written for it.
    pub schema_version: String,
    /// Set to `true` permanently on first publish; never unset thereafter.
    ///
    /// Retention-locked passports must remain publicly accessible for the period
    /// defined in the applicable EU ESPR delegated act (typically 10–15 years after
    /// the product's end of life).
    #[serde(default)]
    pub retention_locked: bool,

    // ── 0.2 data-model fields ──────────────────────────────────────────────
    /// Monotonically increasing version counter. `1` on first publish; increments
    /// each time a new passport version supersedes this one (set on the successor).
    #[serde(default = "default_version")]
    pub version: u32,
    /// The passport ID this record supersedes. `None` for first-version passports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_id: Option<PassportId>,
    /// Cross-operator references to the predecessors this passport derives from
    /// (second-life successor linkage), each typed with the operation that
    /// produced this unit from it. Empty unless this record was issued as a
    /// successor citing source passports held by other operators.
    ///
    /// ✅ COMPLIANCE-PIN: EU 2023/1542, Art. 77(7) (OJ L 191, 28.7.2023, p. 73)
    /// — "linked to the battery passport **or passports** of the original
    /// battery **or batteries**". Plural on both sides: one second-life unit may
    /// derive from several predecessors, which is why this is a `Vec`. See
    /// [`DerivationRef`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<DerivationRef>,
    /// Cross-operator references to the constituent passports this product is
    /// assembled from — its bill of materials. Empty for a unit with no modelled
    /// sub-assemblies. The inverse edge of `derived_from`: `component_refs`
    /// point down to the constituents, `derived_from` points up to the
    /// predecessors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub component_refs: Vec<PassportRef>,
    /// Deadline by which this record must remain accessible. Confirmed against the
    /// verbatim OJ text (Regulation (EU) 2024/1781): **Art. 9(2)(i)** requires the
    /// delegated act to specify "the period during which the digital product
    /// passport is to remain available, which shall correspond to at least the
    /// expected lifetime of a specific product"; **Art. 11(e)** restates this as an
    /// essential requirement, available "including after an insolvency, a
    /// liquidation or a cessation of activity" of the responsible operator. The
    /// separate back-up-copy obligation (via a DPP service provider) is **Art.
    /// 10(4)**, not the retention period itself.
    /// Computed at publish time from `ProductGroupCatalog::retention_years` for the
    /// product group — the single source of the retention obligation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_until: Option<DateTime<Utc>>,
    /// Opaque link to an internal product-template record. Not a legal identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_id: Option<Uuid>,
    /// Customs tariff classification (HS-6, CN-8 or TARIC-10).
    ///
    /// Registration data the EU registry stores and verifies against the ranges
    /// its product group permits. `None` where the product group does not call
    /// for one — the regulation qualifies it "where relevant" — and a registry
    /// that requires it will refuse the registration rather than this node
    /// inventing a classification it cannot derive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commodity_code: Option<crate::identifier::CommodityCode>,
    /// EORI or national economic-operator identifier for the responsible party.
    /// Confirmed against the verbatim OJ text (Regulation (EU) 2024/1781):
    /// **Annex III, point (k)** is the data-content basis — "the name, contact
    /// details and unique operator identifier of the economic operator established
    /// in the Union responsible for carrying out the tasks set out in Article 4 of
    /// Regulation (EU) 2019/1020 [...]"; the identifier-issuance mechanics are
    /// **Art. 12**. (**Art. 13** governs uploading identifiers to the EU registry —
    /// a related but distinct obligation, not the field's basis.) Populated by the
    /// engine from `operator_config`.
    ///
    /// **This is the operator that published the passport, frozen at publish —
    /// not necessarily the operator responsible for it now.** A transfer of
    /// responsibility moves the current operator, and the authoritative record
    /// of that is the passport's [`TransferChain`](crate::transfer::TransferChain)
    /// via `current_operator()`. This field is not rewritten by a transfer and
    /// cannot be: a published passport's content is immutable and this value is
    /// covered by the signature over it. Reading it as "who is responsible
    /// today" is wrong for any passport that has changed hands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_identifier: Option<String>,
    /// Snapshot of the Annex III facility where this product was manufactured or
    /// processed, copied by value at create time. Self-contained so the signed
    /// passport stays a complete record independent of the operator's mutable
    /// facility registry (a retired facility never orphans a published passport).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facility: Option<FacilitySnapshot>,
    /// The eIDAS qualified electronic seal applied to this passport, if any.
    /// `placeholder: true` on the envelope means no legally valid seal exists yet —
    /// consumers must check this flag rather than inferring validity from presence.
    /// `None` until a seal (real or placeholder) has been applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seal: Option<SealedEnvelope>,
}

fn default_version() -> u32 {
    1
}

/// Envelope keys this build has removed, each paired with what replaced it.
///
/// # Why this has to exist
///
/// [`Passport`] deliberately does **not** set `deny_unknown_fields`. A document
/// written by a newer build must stay readable by an older one — that is the
/// point of the envelope's additive-only rule — so an unrecognised key is
/// ignored rather than refused. The cost is that a key which was *removed* is
/// indistinguishable from one this build simply has not learned about yet.
///
/// For a renamed field that is the worst available outcome, and it is not
/// hypothetical: `parentPassportRef` → `derivedFrom` left a stored document
/// carrying the old key deserializing **successfully**, with the new field at
/// its default. The record loads, reports no error, and has silently lost a
/// value its signature still covers — a second-life passport that quietly
/// forgets the predecessors Art. 77(7) requires it to link to.
///
/// [`Passport::from_stored`] checks this list before anything else so that
/// document is refused out loud. A direct `serde_json::from_value::<Passport>`
/// that bypasses `from_stored` still drops the key silently; `from_stored` is
/// the supported way to read a stored document, and this is one of the reasons.
///
/// Entries are permanent. A key removed two renames ago is still a key some
/// stored document may carry.
pub const REMOVED_ENVELOPE_KEYS: &[(&str, &str)] = &[("parentPassportRef", "derivedFrom")];

/// Every key [`Passport`] serialises to, in declaration order.
///
/// # Why this exists
///
/// A consumer that addresses passport JSON *by key* — a JSONB query, an index
/// expression, a database trigger — cannot use the Rust field name and cannot
/// read a Rust constant into SQL. It types the camelCase string literally, and
/// a literal has no relationship to the field it names: rename the field here
/// and the query keeps parsing, keeps running, and silently returns NULL for
/// every row. Nothing fails.
///
/// This is the vocabulary those consumers must check themselves against.
/// `passport_wire_keys_tests` proves it is complete against a fully-populated
/// instance, so a field added or renamed here changes this list, and a consumer
/// gate that compares against it then names the file to fix.
///
/// Serialized (camelCase) names, matching the `Passport` JSON representation.
pub const PASSPORT_WIRE_KEYS: &[&str] = &[
    "id",
    "batchId",
    "productName",
    "productGroup",
    "applicableInstruments",
    "granularity",
    "manufacturer",
    "materials",
    "co2ePerUnit",
    "repairabilityScore",
    "complianceResult",
    "lintResult",
    "productGroupData",
    "status",
    "qrCodeUrl",
    "jwsSignature",
    "publicJwsSignature",
    "disclosureSignatures",
    "createdAt",
    "updatedAt",
    "publishedAt",
    "placedOnMarketDate",
    "schemaVersion",
    "retentionLocked",
    "version",
    "supersedesId",
    "derivedFrom",
    "componentRefs",
    "retentionUntil",
    "productId",
    "commodityCode",
    "operatorIdentifier",
    "facility",
    "seal",
];

/// The [`Passport`] keys that carry a **proof**, never product content.
///
/// # Why these are not a disclosure class
///
/// The obvious modelling is to give each of these an [`crate::disclosure::Audience`] in
/// [`crate::disclosure::PASSPORT_FIELD_DISCLOSURE`] and let redaction handle them like any other
/// field. That was tried and it is a category error, which produced two
/// different bugs at once:
///
/// - `seal`, `publicJwsSignature` and `disclosureSignatures` had **no** entry,
///   so they defaulted to `Public` and a public view carried all three. The last
///   is the damaging one: those are *attached* compact JWS, so each embeds the
///   full redacted body for its own audience — handing an anonymous reader the
///   `public+restricted+individual` entry hands over the restricted payload
///   itself, not a signature over it.
/// - `jwsSignature` **had** an entry, `Conformity`, so an authority received it
///   attached to a body with individual-item data already removed. It covers the
///   *full* payload, so it verifies against nothing that reader was given. The
///   class did not make the answer safer; it made it wrong.
///
/// These are also classed `Conformity` in [`crate::disclosure::PASSPORT_FIELD_DISCLOSURE`] as
/// defence in depth, so a consumer driving the raw filter fails safe. That is a
/// backstop, not the answer: a class can only choose *which* audiences see a
/// field, and the correct answer here is none of them.
///
/// A proof is not data about the product that some audiences may see. It is a
/// statement about a specific sequence of bytes. The rule that actually holds is:
/// **a view is a payload, and whoever serves it attaches the one proof that
/// covers exactly the bytes being sent.** No audience arithmetic can express
/// that, so redaction removes every one of these unconditionally and leaves
/// attaching the right one to the serving layer.
///
/// `passport_every_wire_key_is_classified` fails the build if a new key is added
/// to [`Passport`] without being placed here, in [`crate::disclosure::PASSPORT_FIELD_DISCLOSURE`],
/// or on that test's explicit public allowlist — because a name list is exactly
/// how `seal` came to be missed in the first place.
///
/// Serialized (camelCase) names, matching the `Passport` JSON representation.
pub const PASSPORT_PROOF_FIELDS: &[&str] = &[
    "jwsSignature",
    "publicJwsSignature",
    "disclosureSignatures",
    "seal",
];

/// The only [`Passport`] keys that may change once `retention_locked` is set.
///
/// Retention-locking a passport freezes its *content* (ESPR: the record must
/// remain available and unaltered for the retention period). It does not freeze
/// the record entirely — a passport is still suspended, sealed, re-linted and
/// re-signed after publication, and each of those writes a key here.
///
/// # Why this belongs in core
///
/// Which fields survive the freeze is a statement about what a retained passport
/// guarantees, so it changes when the domain changes — the Golden Rule's test
/// for core ownership. It was previously stated only inside a PostgreSQL trigger
/// and had to be re-typed **in full, five times** as fields were added
/// (`publicJwsSignature`, `lintResult`, `disclosureSignatures`, `seal`), each
/// time as a fresh migration transcribing all ten strings. Nothing said when a
/// sixth was due: a new post-publish-mutable field simply failed at runtime with
/// `ODAL_RETENTION` the first time something tried to write it on a published
/// record.
///
/// A backend enforcing the freeze must derive its list from this value. SQL
/// cannot read a Rust constant, so a trigger necessarily restates it — but a
/// restatement that is *checked against this* is a copy with a guard, not a
/// second source of truth.
///
/// Serialized (camelCase) names, matching the `Passport` JSON representation.
pub const RETENTION_MUTABLE_FIELDS: &[&str] = &[
    // Lifecycle: suspension and archival are lawful after publication.
    "status",
    "publishedAt",
    "retentionLocked",
    "updatedAt",
    // Proofs: re-signing and sealing land after the content is frozen, and each
    // covers the frozen content rather than altering it.
    "jwsSignature",
    "publicJwsSignature",
    "disclosureSignatures",
    "seal",
    // Serving metadata, not passport content.
    "qrCodeUrl",
    // Advisory plausibility output, explicitly re-computable after publish.
    "lintResult",
];

/// The catalog product group key and recorded schema version a stored document's
/// `productGroupData` was written under, read without assuming the document
/// deserializes into the current shape. `None` if either is absent or
/// malformed — [`Passport::from_stored`] then skips upcasting and lets the
/// final deserialize surface whatever is actually wrong.
fn stored_product_group_version(doc: &serde_json::Value) -> Option<(String, String)> {
    let product_group_data = doc.get("productGroupData")?;
    let tag = product_group_data.get("productGroup")?.as_str()?;
    let product_group_key = ProductGroup::from_wire_tag(tag).catalog_key().to_owned();
    let recorded = doc.get("schemaVersion")?.as_str()?.to_owned();
    Some((product_group_key, recorded))
}

impl Passport {
    /// Deserialize a passport as it was actually stored. Tries the direct,
    /// current-shape deserialize first — most schema evolution is additive
    /// and a document written under an older `schemaVersion` reads directly
    /// with no transform needed, exactly as before this method existed. Only
    /// on failure does it fall back to upcasting `productGroupData` through the
    /// registered lens chain and retrying, so a version gap that needs no
    /// lens (the common case) never pays for one.
    ///
    /// The fallback upcasts as far toward the product group's current version as the
    /// registered lenses reach ([`crate::schemas::lens::LensRegistry::upcast_toward`]),
    /// not to a chain landing on it exactly. A product group whose schema has moved on
    /// additively since its last lens has no hop ending at the current version,
    /// and requiring one would refuse every document the lenses it *does* have
    /// would have made readable. The additive remainder needs no transform by
    /// definition, so the deserialize below closes it.
    ///
    /// **Envelope fields (everything outside `productGroupData`) are not lensed —
    /// deliberately, not an oversight.** A lens transforms one product group's
    /// sub-object; an envelope field is shared by every product group's documents, so
    /// a non-additive envelope change would need a transform over the *whole*
    /// document, and getting that wrong silently corrupts every product group at
    /// once rather than one. The envelope's compatibility rule is simpler and
    /// stricter instead: additive only — see [`Passport::schema_version`]'s doc
    /// comment, which also records the two renames that have broken it and why
    /// that licence is temporary. A stored document that
    /// still fails to deserialize after its `productGroupData` has been upcast is
    /// therefore either genuinely malformed or violates that rule, and this
    /// method does not try to guess which.
    ///
    /// Two distinct failure shapes, both typed rather than a generic error:
    /// - [`crate::error::dpp::DppError::SchemaIncompatible`] — the recorded `schemaVersion` is
    ///   older than current and no registered lens bridges any of the gap.
    ///   This is not always fixable by writing one: a required field the
    ///   document predates (no source data anywhere in the document to derive
    ///   it from) has no honest transform, and this crate will not synthesize
    ///   one.
    /// - [`crate::error::dpp::DppError::Serialisation`] — the direct attempt failed for a reason
    ///   unrelated to a bridgeable version gap (no product group data, product group
    ///   unknown to the catalog, already at the current version, or the
    ///   upcast document still does not match the current shape).
    pub fn from_stored(
        doc: serde_json::Value,
        lenses: &crate::schemas::lens::LensRegistry,
        catalog: &crate::catalog::ProductGroupCatalog,
    ) -> Result<Self, crate::error::dpp::DppError> {
        use crate::error::dpp::DppError;
        use serde::Deserialize as _;

        // Before the direct attempt, not after: a document carrying a removed
        // key deserializes *successfully*, so anything downstream of the happy
        // path would never see it.
        if let Some(object) = doc.as_object() {
            for &(removed, replacement) in REMOVED_ENVELOPE_KEYS {
                if object.contains_key(removed) {
                    return Err(DppError::RemovedEnvelopeKey {
                        removed,
                        replacement,
                    });
                }
            }
        }

        let direct_err = match Self::deserialize(&doc) {
            Ok(passport) => return Ok(passport),
            Err(e) => e,
        };

        let Some((product_group_key, recorded)) = stored_product_group_version(&doc) else {
            return Err(DppError::Serialisation(direct_err.to_string()));
        };
        let Some(current) = catalog.current_schema_version(&product_group_key) else {
            return Err(DppError::Serialisation(direct_err.to_string()));
        };
        if recorded == current {
            return Err(DppError::Serialisation(direct_err.to_string()));
        }

        let product_group_data = doc["productGroupData"].clone();
        let derived = lenses.upcast_str_toward(
            &product_group_key,
            &product_group_data,
            &recorded,
            current,
        )?;
        let mut doc = doc;
        doc["productGroupData"] = derived.data;

        serde_json::from_value(doc).map_err(|e| DppError::Serialisation(e.to_string()))
    }

    /// Validate the passport's own field invariants.
    ///
    /// Checks:
    /// - `product_name` is non-empty
    /// - `manufacturer.name` is non-empty
    /// - `manufacturer.address` is non-empty
    /// - `schema_version` follows semver pattern (x.y.z)
    /// - `co2e_per_unit` is non-negative if present
    /// - `repairability_score` is in range [0.0, 10.0] if present
    /// - `product_group_data.product group()` matches `self.product_group` if present
    /// - for `ProductGroup::UnsoldGoods`, the disclosure carries at least one
    ///   product line (Impl. Reg. (EU) 2026/2 Annex I). No Annex VII scope check:
    ///   that is Art. 25's destruction ban, not Art. 24's disclosure duty
    ///
    /// **This does not validate `product_group_data` against its JSON Schema**,
    /// and does not run the cross-field regulatory rules. That pass needs the
    /// versioned schema registry — and through it `jsonschema` and a blocking
    /// HTTP client — which an aggregate stating its own invariants must not
    /// depend on. It is why this method is the same on every target, `wasm32`
    /// included.
    ///
    /// **For both halves, call [`crate::validation::validate_passport`]**, which
    /// runs this and then [`crate::validation::validate_product_group_data`].
    pub fn validate(&self) -> Result<(), crate::error::dpp::DppError> {
        use crate::field_error::{FieldError, ValidationErrors};

        let mut errors: Vec<FieldError> = Vec::new();

        if self.product_name.trim().is_empty() {
            errors.push(FieldError {
                field: "/productName".to_owned(),
                message: "product_name must not be empty".to_owned(),
            });
        }
        if self.manufacturer.name.trim().is_empty() {
            errors.push(FieldError {
                field: "/manufacturer/name".to_owned(),
                message: "manufacturer.name must not be empty".to_owned(),
            });
        }
        if self.manufacturer.address.trim().is_empty() {
            errors.push(FieldError {
                field: "/manufacturer/address".to_owned(),
                message: "manufacturer.address must not be empty".to_owned(),
            });
        }

        // Must parse as strict semver (major.minor.patch, optional pre-release
        // / build metadata). A hand-rolled digit check let ".5.0" (empty major)
        // and "1.0.abc" (non-numeric patch) through — both then fail
        // `semver::Version` parsing at schema resolution and silently skip
        // schema validation, so reject them here rather than downstream.
        if self.schema_version.parse::<semver::Version>().is_err() {
            errors.push(FieldError {
                field: "/schemaVersion".to_owned(),
                message: "schema_version must be valid semver (e.g. 1.0.0)".to_owned(),
            });
        }

        if let Some(ref cf) = self.co2e_per_unit
            && cf.value_kg < 0.0
        {
            errors.push(FieldError {
                field: "/co2ePerUnit".to_owned(),
                message: "co2e_per_unit must not be negative".to_owned(),
            });
        }

        if let Some(ref rs) = self.repairability_score
            && !(0.0..=10.0).contains(&rs.overall)
        {
            errors.push(FieldError {
                field: "/repairabilityScore".to_owned(),
                message: "repairability_score must be between 0.0 and 10.0".to_owned(),
            });
        }

        // The declared product group must match the product group of the typed data, if present.
        if let Some(ref data) = self.product_group_data
            && data.product_group() != self.product_group
        {
            errors.push(FieldError {
                field: "/product_group".to_owned(),
                message: "product_group must match product_group_data's product_group".to_owned(),
            });
        }

        // Two fields carry the placing-on-market date: this envelope one, which
        // every product group has and which a determination reads, and battery's own,
        // which shipped first and is in released schemas. They must not
        // disagree — the date selects which law binds the product, so two
        // answers is two different sets of obligations, and nothing downstream
        // can tell which was meant.
        //
        // Not a duplicated *regulated* field: `placedOnMarketDate` is absent
        // from the Commission's battery data-point guidance. It was added to
        // drive the Art. 8 phase determination, which is why promoting it here
        // costs no Annex XIII coverage.
        if let Some(ProductGroupData::Battery(battery)) = &self.product_group_data
            && let (Some(envelope), Some(product_group)) =
                (self.placed_on_market_date, battery.placed_on_market_date)
            && envelope != product_group
        {
            errors.push(FieldError {
                field: "/productGroupData/placedOnMarketDate".to_owned(),
                message: format!(
                    "placed_on_market_date disagrees with the passport's own \
                     ({product_group} vs {envelope}); the date fixes which law governs \
                     this battery, so it cannot have two values"
                ),
            });
        }

        // An unsold-goods record is a disclosure by an undertaking over a
        // financial year, not a product placed on the market, so the envelope's
        // `commodity_code` has nothing to describe: the categories are on the
        // lines, and there are many of them.
        //
        // This deliberately no longer requires Annex VII scope. **Art. 24
        // (disclosure) and Art. 25 (destruction ban) have different scopes** —
        // the ban reaches Annex VII's apparel and footwear, while the disclosure
        // reaches discarded unsold *consumer products* generally, which Impl.
        // Reg. (EU) 2026/2 Annex II illustrates across 45 CN headings from soap
        // to refrigerators. Requiring Annex VII here rejected every lawful
        // disclosure outside apparel and footwear.
        if self.product_group == ProductGroup::UnsoldGoods
            && let Some(ProductGroupData::UnsoldGoods(report)) = &self.product_group_data
            && report.lines.is_empty()
        {
            errors.push(FieldError {
                field: "/productGroupData/lines".to_owned(),
                message: "an unsold-goods disclosure must carry at least one product line \
                          (Impl. Reg. (EU) 2026/2 Annex I)"
                    .to_owned(),
            });
        }

        // Schema conformance is deliberately NOT checked here. It needs the
        // versioned schema registry, which drags `jsonschema` and through it a
        // blocking HTTP client, and an aggregate that cannot state its own
        // invariants without a network stack in the tree is the wrong shape.
        // `validation::validate_passport` runs both halves; this method is the
        // invariants alone, and is the same on every target.
        if errors.is_empty() {
            Ok(())
        } else {
            Err(crate::error::dpp::DppError::Validation(ValidationErrors {
                errors,
            }))
        }
    }

    /// Transition the passport to a new status, enforcing the state machine.
    ///
    /// Valid transitions:
    /// ```text
    /// Draft → Published | Archived
    /// Published → Suspended | Archived
    /// Suspended → Published | Archived
    /// ```
    ///
    /// On the first `Draft → Published` transition this method also:
    /// - Sets `retention_locked = true` (ESPR retention obligation).
    /// - Sets `published_at` to the current timestamp.
    /// - Updates `updated_at`.
    pub fn transition_to(
        &mut self,
        next: PassportStatus,
    ) -> Result<(), crate::error::dpp::DppError> {
        if !self.status.can_transition_to(&next) {
            return Err(crate::error::dpp::DppError::InvalidTransition {
                current: self.status.to_string(),
                required: next.to_string(),
            });
        }

        let now = chrono::Utc::now();

        // First publish: gate on mandatory content, then lock retention and
        // record the timestamp.
        if next == PassportStatus::Published && self.published_at.is_none() {
            self.check_mandatory_content()?;
            self.retention_locked = true;
            self.published_at = Some(now);
        }

        self.status = next;
        self.updated_at = now;
        Ok(())
    }

    /// Refuse a first publish that omits content the battery's category makes
    /// mandatory.
    ///
    /// # Asking without attempting
    ///
    /// Public so a caller can *preview* the gate. [`transition_to`] runs this
    /// same function, so a preview and the refusal it predicts cannot drift —
    /// and because it takes `&self` and returns the same [`DppError`], a
    /// consumer can ask the question without a state change and render the
    /// answer byte-identically.
    ///
    /// Being able to ask is not being able to decline: the gate still runs
    /// inside `transition_to`, and `status`/`published_at` remain unsettable by
    /// hand, so there is no path to publishing that skips it.
    ///
    /// A failure names **every** missing field at once rather than the first,
    /// so one call is a complete answer.
    ///
    /// [`transition_to`]: Passport::transition_to
    /// [`DppError`]: crate::error::dpp::DppError
    ///
    /// # Why this is a hard gate and not a lint
    ///
    /// A passport missing content the law requires is not a passport with a
    /// quality problem — it is one that should not exist. Putting the check in
    /// `dpp-domain` rather than in a consumer means no caller can opt out of
    /// it: an engine-side check would be bypassed by the next engine.
    ///
    /// # Why only on the *first* publish
    ///
    /// `transition_to` also runs on `Suspended → Published`. Gating a republish
    /// would let a later change to the requirements table strand a passport
    /// that was lawfully published under the earlier one — the same hazard as a
    /// lens that refuses, and worse, because the operator cannot fix a
    /// retention-locked document. The content is fixed at first publish; that is
    /// where it is judged.
    ///
    /// # Scope
    ///
    /// Battery only, and only for the three categories the source covers. A
    /// portable or SLI battery is **ungated** — the Commission's guidance says
    /// nothing about them, and inventing a requirement it declines to state
    /// would be the defect this crate exists to avoid. That is a real hole and
    /// it is deliberate; it closes when a source covering those categories
    /// exists.
    pub fn check_mandatory_content(&self) -> Result<(), crate::error::dpp::DppError> {
        use crate::field_error::{FieldError, ValidationErrors};

        if self.product_group != crate::product_group::ProductGroup::Battery {
            return Ok(());
        }
        let Some(data) = self.product_group_data.as_ref() else {
            return Err(crate::error::dpp::DppError::Validation(ValidationErrors {
                errors: vec![FieldError {
                    field: "/productGroupData".to_owned(),
                    message: "a battery passport cannot be published without product_group data"
                        .to_owned(),
                }],
            }));
        };
        let Ok(value) = serde_json::to_value(data) else {
            return Ok(());
        };
        let Some(battery_type) = value.get("batteryType").and_then(serde_json::Value::as_str)
        else {
            // batteryType is required by the schema from v2.5.0; if it is absent
            // here the schema check is the right place to say so.
            return Ok(());
        };

        // A key present but null is absent: `skip_serializing_if` means a `None`
        // never reaches the wire, so an explicit null came from somewhere else
        // and carries no value either way.
        let missing: Vec<FieldError> =
            dpp_rules::batteries::passport_content::mandatory_fields(battery_type)
                .filter(|f| value.get(*f).is_none_or(serde_json::Value::is_null))
                .map(|f| FieldError {
                    field: format!("/productGroupData/{f}"),
                    message: format!(
                        "'{f}' is mandatory for a '{battery_type}' battery and is absent; \
                         a passport omitting it does not carry the content the Battery \
                         Regulation requires of this category"
                    ),
                })
                .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(crate::error::dpp::DppError::Validation(ValidationErrors {
                errors: missing,
            }))
        }
    }
}
