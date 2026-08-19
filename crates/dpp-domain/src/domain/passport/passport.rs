//! The [`Passport`] aggregate root.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    FacilitySnapshot, ManufacturerInfo, MaterialEntry, PassportId, PassportRef, PassportView,
};
use crate::domain::{
    identity::{Audience, Disclosure, PASSPORT_FIELD_DISCLOSURE},
    lint::LintResult,
    sector::{CarbonFootprint, RepairabilityScore, Sector, SectorData},
    status::PassportStatus,
};
use crate::ports::compliance::ComplianceResult;
use crate::ports::seal::SealedEnvelope;

/// The canonical Digital Product Passport record as defined by EU ESPR.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Passport {
    pub id: PassportId,
    /// Optional batch or lot identifier.
    pub batch_id: Option<String>,
    pub product_name: String,
    /// EU ESPR sector — the delegated-act bucket that selects the applicable
    /// schema and plugin. (Replaces the former misnamed `product_category`
    /// field, which actually held a sector.)
    pub sector: Sector,
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
    /// until a determination is computed (e.g. a sector with no plugin loaded).
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
    /// Typed, sector-specific DPP data (EU Battery Regulation, Textile DPP, etc.).
    ///
    /// `None` for passports where sector-specific data has not yet been supplied.
    /// Set this field when publishing to ensure regulatory compliance validation.
    pub sector_data: Option<SectorData>,
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
    /// [`disclosure_key`](crate::domain::identity::disclosure_key) — e.g.
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
    /// Envelope-level rather than per-sector because the triggering event is
    /// not sector-specific: ESPR attaches its duties at placing on the market
    /// for every product group, as do Regulation (EU) 2023/1542 Art. 7, 8 and
    /// 10 for batteries. It lived only on `BatteryData` before, which made the
    /// governing law underivable for the other eleven sectors.
    ///
    /// `None` means the date was not declared, which is **not** a licence to
    /// substitute the current date. A determination that depends on it has no
    /// answer, and saying so is the answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed_on_market_date: Option<NaiveDate>,
    /// Semantic version of the *sector* schema used to validate this record.
    ///
    /// Scoped to `sector_data` only — there is no equivalent version for the
    /// envelope fields on this struct. [`Passport::from_stored`] uses this to
    /// decide whether `sector_data` needs upcasting through a lens before
    /// this record can be re-read. Envelope fields have no such escape hatch
    /// and never will: a lens transforms one sector's sub-object, but an
    /// envelope field is shared by every sector's stored documents, so a
    /// non-additive envelope change would need a transform over the whole
    /// document — one mistake there corrupts every sector at once, not one.
    /// The envelope's rule is therefore additive-only, permanently, with no
    /// exception path: `Option<T>` + `#[serde(default)]`, or a rename that
    /// keeps accepting the old key, never a bare requirement added to an
    /// existing field.
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
    /// Cross-operator reference to the predecessor this passport derives from
    /// (second-life successor linkage). `None` unless this record was issued as a
    /// successor citing a source passport held by another operator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_passport_ref: Option<PassportRef>,
    /// Cross-operator references to the constituent passports this product is
    /// assembled from — its bill of materials. Empty for a unit with no modelled
    /// sub-assemblies. The inverse edge of `parent_passport_ref`: `component_refs`
    /// point down to many constituents, `parent_passport_ref` points up to one
    /// predecessor.
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
    /// Computed at publish time from `SectorCatalog::retention_years` for the
    /// sector — the single source of the retention obligation.
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
    pub commodity_code: Option<crate::domain::commodity_code::CommodityCode>,
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
    /// of that is the passport's [`TransferChain`](crate::domain::transfer::TransferChain)
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

/// The catalog sector key and recorded schema version a stored document's
/// `sectorData` was written under, read without assuming the document
/// deserializes into the current shape. `None` if either is absent or
/// malformed — [`Passport::from_stored`] then skips upcasting and lets the
/// final deserialize surface whatever is actually wrong.
fn stored_sector_version(doc: &serde_json::Value) -> Option<(String, String)> {
    let sector_data = doc.get("sectorData")?;
    let tag = sector_data.get("sector")?.as_str()?;
    let sector_key = Sector::from_wire_tag(tag).catalog_key().to_owned();
    let recorded = doc.get("schemaVersion")?.as_str()?.to_owned();
    Some((sector_key, recorded))
}

impl Passport {
    /// Deserialize a passport as it was actually stored. Tries the direct,
    /// current-shape deserialize first — most schema evolution is additive
    /// and a document written under an older `schemaVersion` reads directly
    /// with no transform needed, exactly as before this method existed. Only
    /// on failure does it fall back to upcasting `sectorData` through the
    /// registered lens chain and retrying, so a version gap that needs no
    /// lens (the common case) never pays for one.
    ///
    /// The fallback upcasts as far toward the sector's current version as the
    /// registered lenses reach ([`crate::schemas::lens::LensRegistry::upcast_toward`]),
    /// not to a chain landing on it exactly. A sector whose schema has moved on
    /// additively since its last lens has no hop ending at the current version,
    /// and requiring one would refuse every document the lenses it *does* have
    /// would have made readable. The additive remainder needs no transform by
    /// definition, so the deserialize below closes it.
    ///
    /// **Envelope fields (everything outside `sectorData`) are not lensed —
    /// deliberately, not an oversight.** A lens transforms one sector's
    /// sub-object; an envelope field is shared by every sector's documents, so
    /// a non-additive envelope change would need a transform over the *whole*
    /// document, and getting that wrong silently corrupts every sector at
    /// once rather than one. The envelope's compatibility rule is simpler and
    /// stricter instead: additive only, permanently, no exceptions — see
    /// [`Passport::schema_version`]'s doc comment. A stored document that
    /// still fails to deserialize after its `sectorData` has been upcast is
    /// therefore either genuinely malformed or violates that rule, and this
    /// method does not try to guess which.
    ///
    /// Two distinct failure shapes, both typed rather than a generic error:
    /// - [`crate::domain::error::DppError::SchemaIncompatible`] — the recorded `schemaVersion` is
    ///   older than current and no registered lens bridges any of the gap.
    ///   This is not always fixable by writing one: a required field the
    ///   document predates (no source data anywhere in the document to derive
    ///   it from) has no honest transform, and this crate will not synthesize
    ///   one.
    /// - [`crate::domain::error::DppError::Serialisation`] — the direct attempt failed for a reason
    ///   unrelated to a bridgeable version gap (no sector data, sector
    ///   unknown to the catalog, already at the current version, or the
    ///   upcast document still does not match the current shape).
    pub fn from_stored(
        doc: serde_json::Value,
        lenses: &crate::schemas::lens::LensRegistry,
        catalog: &crate::catalog::SectorCatalog,
    ) -> Result<Self, crate::domain::error::DppError> {
        use crate::domain::error::DppError;
        use serde::Deserialize as _;

        let direct_err = match Self::deserialize(&doc) {
            Ok(passport) => return Ok(passport),
            Err(e) => e,
        };

        let Some((sector_key, recorded)) = stored_sector_version(&doc) else {
            return Err(DppError::Serialisation(direct_err.to_string()));
        };
        let Some(current) = catalog.current_schema_version(&sector_key) else {
            return Err(DppError::Serialisation(direct_err.to_string()));
        };
        if recorded == current {
            return Err(DppError::Serialisation(direct_err.to_string()));
        }

        let sector_data = doc["sectorData"].clone();
        let derived = lenses.upcast_str_toward(&sector_key, &sector_data, &recorded, current)?;
        let mut doc = doc;
        doc["sectorData"] = derived.data;

        serde_json::from_value(doc).map_err(|e| DppError::Serialisation(e.to_string()))
    }

    /// Validate passport fields for structural correctness and sector-data integrity.
    ///
    /// Checks:
    /// - `product_name` is non-empty
    /// - `manufacturer.name` is non-empty
    /// - `manufacturer.address` is non-empty
    /// - `schema_version` follows semver pattern (x.y.z)
    /// - `co2e_per_unit` is non-negative if present
    /// - `repairability_score` is in range [0.0, 10.0] if present
    /// - `sector_data.sector()` matches `self.sector` if present
    /// - for `Sector::UnsoldGoods`, `commodity_code` is present and within
    ///   ESPR Annex VII scope (apparel & clothing accessories, or footwear),
    ///   and `sector_data.product_category` (when present) agrees with the
    ///   Annex VII heading the commodity code falls under
    /// - `sector_data` passes JSON Schema + cross-field rules via
    ///   [`crate::domain::validation::validate_sector_data`] (non-wasm32 only)
    pub fn validate(&self) -> Result<(), crate::domain::error::DppError> {
        use crate::domain::field_error::{FieldError, ValidationErrors};

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

        // The declared sector must match the sector of the typed data, if present.
        if let Some(ref data) = self.sector_data
            && data.sector() != self.sector
        {
            errors.push(FieldError {
                field: "/sector".to_owned(),
                message: "sector must match sector_data's sector".to_owned(),
            });
        }

        // Two fields carry the placing-on-market date: this envelope one, which
        // every sector has and which a determination reads, and battery's own,
        // which shipped first and is in released schemas. They must not
        // disagree — the date selects which law binds the product, so two
        // answers is two different sets of obligations, and nothing downstream
        // can tell which was meant.
        //
        // Not a duplicated *regulated* field: `placedOnMarketDate` is absent
        // from the Commission's battery data-point guidance. It was added to
        // drive the Art. 8 phase determination, which is why promoting it here
        // costs no Annex XIII coverage.
        if let Some(SectorData::Battery(battery)) = &self.sector_data
            && let (Some(envelope), Some(sector)) =
                (self.placed_on_market_date, battery.placed_on_market_date)
            && envelope != sector
        {
            errors.push(FieldError {
                field: "/sectorData/placedOnMarketDate".to_owned(),
                message: format!(
                    "placed_on_market_date disagrees with the passport's own \
                     ({sector} vs {envelope}); the date fixes which law governs \
                     this battery, so it cannot have two values"
                ),
            });
        }

        // ESPR Annex VII eligibility: an unsold-goods passport must declare a
        // commodity code within Annex VII's two headings (apparel & clothing
        // accessories, or footwear) — a passport cannot claim this sector for
        // a product the destruction ban does not cover. When sector_data is
        // also present, its own product_category word must agree with the
        // heading the commodity code actually falls under — two fields
        // describing the same product must not contradict each other.
        if self.sector == Sector::UnsoldGoods {
            match &self.commodity_code {
                None => errors.push(FieldError {
                    field: "/commodityCode".to_owned(),
                    message: "commodity_code is required for sector unsoldGoods (ESPR Annex VII scope check)".to_owned(),
                }),
                Some(code) => match crate::domain::sector::unsold_goods_annex_vii_heading(code.as_str()) {
                    None => errors.push(FieldError {
                        field: "/commodityCode".to_owned(),
                        message: "commodity_code is not within ESPR Annex VII scope (apparel/clothing accessories or footwear)".to_owned(),
                    }),
                    Some(heading) => {
                        if let Some(SectorData::UnsoldGoods(report)) = &self.sector_data
                            && !crate::domain::sector::unsold_goods_category_matches_heading(
                                &report.product_category,
                                heading,
                            )
                        {
                            errors.push(FieldError {
                                field: "/sectorData/productCategory".to_owned(),
                                message: "product_category does not match the Annex VII heading commodity_code falls under".to_owned(),
                            });
                        }
                    }
                },
            }
        }

        // Sector-data validation: JSON Schema + cross-field rules (fibre sum, SVHC, etc.).
        // Excluded from wasm32 builds because jsonschema depends on reqwest's blocking API.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref data) = self.sector_data
            && let Err(ve) = crate::domain::validation::validate_sector_data(data)
        {
            errors.extend(ve.errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(crate::domain::error::DppError::Validation(
                ValidationErrors { errors },
            ))
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
    ) -> Result<(), crate::domain::error::DppError> {
        if !self.status.can_transition_to(&next) {
            return Err(crate::domain::error::DppError::InvalidTransition {
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
    fn check_mandatory_content(&self) -> Result<(), crate::domain::error::DppError> {
        use crate::domain::field_error::{FieldError, ValidationErrors};

        if self.sector != crate::domain::sector::Sector::Battery {
            return Ok(());
        }
        let Some(data) = self.sector_data.as_ref() else {
            return Err(crate::domain::error::DppError::Validation(
                ValidationErrors {
                    errors: vec![FieldError {
                        field: "/sectorData".to_owned(),
                        message: "a battery passport cannot be published without sector data"
                            .to_owned(),
                    }],
                },
            ));
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
                    field: format!("/sectorData/{f}"),
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
            Err(crate::domain::error::DppError::Validation(
                ValidationErrors { errors: missing },
            ))
        }
    }

    /// Return an audience-filtered JSON view of this passport.
    ///
    /// NOTE: this is a self-contained domain convenience, **not** the authoritative
    /// public view. The payload that is signed (`publicJwsSignature`) and served on
    /// the public route is produced by the `dpp-crypto` policy engine (vault
    /// `public_view`), which fails closed on unknown sectors. Do not wire this into
    /// the public-serving path expecting byte-parity with the signed view.
    ///
    /// Top-level fields are removed according to
    /// [`PASSPORT_FIELD_DISCLOSURE`]
    /// — the single source for this fact, shared with the crypto layer's
    /// `SectorAccessPolicy::passport_default()`. At present:
    /// - `Restricted`: `batchId`, `lintResult`
    /// - `Conformity`: `jwsSignature`, `retentionLocked`
    ///
    /// Anything absent from that table is `Public`.
    ///
    /// `sectorData`, when present, is independently redacted via
    /// [`crate::domain::sector::redact_sector_data`] against the sector descriptor
    /// from `catalog`. If the sector is not in the catalog, sector data is
    /// **withheld** from every audience except `Authority` (fail-closed): without
    /// the descriptor's per-field disclosure classes the domain layer cannot tell
    /// which fields are safe to expose, so it exposes none.
    pub fn redact(
        &self,
        audience: Audience,
        catalog: &crate::catalog::SectorCatalog,
    ) -> PassportView {
        let mut value = match serde_json::to_value(self) {
            Ok(v) => v,
            Err(_) => return PassportView(serde_json::Value::Null),
        };

        if let Some(obj) = value.as_object_mut() {
            // Driven by the shared table, not a second hand-written list —
            // the two used to disagree about `lintResult`.
            for (field, class) in PASSPORT_FIELD_DISCLOSURE {
                if !audience.may_see(*class) {
                    obj.remove(*field);
                }
            }
            // Re-redact sectorData using the catalog's per-field disclosure map.
            if let Some(ref sd) = self.sector_data {
                let sector = sd.sector();
                let key = sector.catalog_key();
                let redacted = if let Some(descriptor) = catalog.get(key) {
                    crate::domain::sector::redact_sector_data(sd, audience, descriptor)
                } else if audience.may_see(Disclosure::Conformity) {
                    // Unknown sector: the full payload is only safe for the
                    // audience that already sees conformity evidence.
                    serde_json::to_value(sd).unwrap_or(serde_json::Value::Null)
                } else {
                    // Fail closed: without per-field classes we cannot tell
                    // which fields are restricted, so withhold sector data
                    // entirely rather than leak it.
                    serde_json::Value::Null
                };
                obj.insert("sectorData".into(), redacted);
            } else {
                obj.remove("sectorData");
            }
        }

        PassportView(value)
    }
}
