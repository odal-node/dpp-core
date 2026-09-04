# Digital Product Passport — Data Model

This document defines the canonical data structures for all Digital Product Passports. It is the authoritative reference for domain types in `dpp-domain`, JSON Schema fields, and Verifiable Credential payloads.

> **Last updated**: 2026-08-13 (aligned to `BatteryData` v2.6.0, `TextileData` v1.2.0,
> `ElectronicsData` v1.2.0). Field-level detail lives in the types and schemas;
> this document describes shape and rules, not every column.

---

## 1. Design Principles

1. **Regulation-anchored**: Every mandatory field maps to a specific article in a delegated act or the ESPR framework. Non-regulatory fields are labelled `internal`.
2. **Schema-versioned**: Every passport record carries a `schema_version` field. Migrating to a new regulatory schema does not invalidate old passports.
3. **Product group-extensible**: A base `Passport` struct holds cross-product group fields. Product group-specific data is stored as a typed enum variant (`ProductGroupData`). Adding a new product group requires a new variant and a new JSON schema — no changes to the base type.
4. **Provisional vs. Strict**: Fields derived from adopted delegated acts are **strict** (legally mandatory). Fields from working group drafts are **provisional** and may change when the delegated act is finalised.
5. **Append-only lifecycle**: No DPP record is ever deleted. State transitions are append-only audit log entries. Archival is a state, not deletion.

---

## 2. Passport Lifecycle

```
Draft  -->  Published (Active)  -->  Suspended  -->  Archived
  |                                      |
  +--------------------------------------+
                  (can also archive directly)
```

| State | Wire name | Publicly Resolvable | Meaning |
|---|---|---|---|
| `Draft` | `"draft"` | No | Under construction; not yet visible via QR |
| `Published` | `"active"` | Yes | Published; signed; JWS signature is set; `retention_locked = true` |
| `Suspended` | `"suspended"` | No | Temporarily hidden — recall, regulatory hold, or dispute; JWS is preserved |
| `Archived` | `"archived"` | Read-only | Product end-of-life; retained for regulatory record-keeping; immutable |

**Transition rules:**
- `Draft -> Published`: Requires all `strict` fields for the declared product group to be present and valid.
- `Published -> Suspended`: Requires an authenticated action with a stated reason.
- `Suspended -> Published`: Requires re-validation of all `strict` fields.
- `Any -> Archived`: Irreversible.

Custom serde: domain `Published` serialises to wire `"active"` (and back). This matches the EU registry's terminology.

---

## 3. Core Fields

### 3.1 Base Passport (`Passport` struct)

All DPPs — regardless of product group — carry these fields. Source:
`dpp-domain/src/domain/passport/passport.rs`.

**`PASSPORT_WIRE_KEYS` in that file is the authority**, not this table. It is a
`const` the tests assert against, so it cannot drift; the table below is a
reading aid and is only as fresh as its last edit. If the two disagree, the
constant is right.

| Field | Rust Type | JSON name | Description |
|---|---|---|---|
| `id` | `PassportId` | `"id"` | Unique passport identifier |
| `batch_id` | `Option<String>` | `"batchId"` | Optional batch or lot identifier (ESPR Art. 9) |
| `product_name` | `String` | `"productName"` | Human-readable product name (ESPR Art. 9) |
| `product_group` | `ProductGroup` | `"productGroup"` | EU ESPR product group — the **dispatch key** (`battery`, `textile`, …). Selects schema + plugin. |
| `applicable_instruments` | `Vec<InstrumentRef>` | `"applicableInstruments"` | The acts that applied at issuance, **recorded not computed**, and immutable thereafter (see §3.5) |
| `granularity` | `Option<Granularity>` | `"granularity"` | Model / batch / item level, an ESPR Art. 9(2)(d) delegated-act decision. `None` where no act has fixed one — the position of every ESPR product group today |
| `manufacturer` | `ManufacturerInfo` | `"manufacturer"` | Nested: name, address, optional did:web URL |
| `materials` | `Vec<MaterialEntry>` | `"materials"` | Bill of materials entries |
| `co2e_per_unit` | `Option<CarbonFootprint>` | `"co2ePerUnit"` | CO₂e per unit — may be set by the compliance engine |
| `repairability_score` | `Option<RepairabilityScore>` | `"repairabilityScore"` | Structured `{overall, criteria}`, not a bare number |
| `compliance_result` | `Option<ComplianceResult>` | `"complianceResult"` | Outcome of the last determination |
| `lint_result` | `Option<LintResult>` | `"lintResult"` | Advisory findings. `None` until a lint pass has run |
| `product_group_data` | `Option<ProductGroupData>` | `"productGroupData"` | Typed product-group-specific data (tagged enum) |
| `status` | `PassportStatus` | `"status"` | Lifecycle state (see §2) |
| `qr_code_url` | `Option<String>` | `"qrCodeUrl"` | Public URL for QR code resolution |
| `jws_signature` | `Option<String>` | `"jwsSignature"` | Compact JWS over the **full** canonical payload (Ed25519) |
| `public_jws_signature` | `Option<String>` | `"publicJwsSignature"` | JWS over the **public projection** — a different redaction, so never interchangeable with the above |
| `disclosure_signatures` | `BTreeMap<String, String>` | `"disclosureSignatures"` | Per-audience signatures, keyed by audience |
| `created_at` | `DateTime<Utc>` | `"createdAt"` | Record creation timestamp |
| `updated_at` | `DateTime<Utc>` | `"updatedAt"` | Last modification timestamp |
| `published_at` | `Option<DateTime<Utc>>` | `"publishedAt"` | First publish timestamp |
| `placed_on_market_date` | `Option<NaiveDate>` | `"placedOnMarketDate"` | Fixes which law governs. Never defaulted to today — a determination depending on an absent value has no answer, and saying so is the answer |
| `schema_version` | `String` | `"schemaVersion"` | Semver of the product group schema used for validation |
| `retention_locked` | `bool` | `"retentionLocked"` | Set permanently on first publish; prevents deletion |
| `version` | `u32` | `"version"` | Monotonic counter; `1` on first publish |
| `supersedes_id` | `Option<PassportId>` | `"supersedesId"` | The passport this record supersedes. `None` for first versions |
| `derived_from` | `Vec<DerivationRef>` | `"derivedFrom"` | Cross-operator predecessors this unit derives from, each typed with its Art. 77(7) operation. Plural because the article is. Omitted when empty |
| `component_refs` | `Vec<PassportRef>` | `"componentRefs"` | Cross-operator references to constituent passports. Omitted when empty |
| `retention_until` | `Option<DateTime<Utc>>` | `"retentionUntil"` | Computed at publish from the instrument bindings' retention fold |
| `product_id` | `Option<Uuid>` | `"productId"` | Opaque link to an internal product-template record. **Not a legal identifier** |
| `commodity_code` | `Option<CommodityCode>` | `"commodityCode"` | CN code. Absent rather than guessed — a registry requiring it refuses the registration instead of this node inventing a classification |
| `operator_identifier` | `Option<String>` | `"operatorIdentifier"` | The operator **at signing time**, covered by the signature. Reading it as "who is responsible today" is wrong for any passport that has changed hands |
| `facility` | `Option<FacilitySnapshot>` | `"facility"` | A snapshot, so a retired facility never orphans a published passport |
| `seal` | `Option<SealedEnvelope>` | `"seal"` | eIDAS qualified electronic seal over `jws_signature` |

### 3.2 ManufacturerInfo

Nested struct within `Passport.manufacturer`.

| Field | Rust Type | JSON name | Description |
|---|---|---|---|
| `name` | `String` | `"name"` | Legal entity name |
| `address` | `String` | `"address"` | Business address or country code |
| `did_web_url` | `Option<String>` | `"didWebUrl"` | `did:web` URL for DID document resolution |

### 3.3 MaterialEntry

Elements of `Passport.materials` — bill of materials entries.

| Field | Rust Type | JSON name | Description |
|---|---|---|---|
| `name` | `String` | `"name"` | Material name |
| `weight_kg` | `f64` | `"weightKg"` | Weight in kg |
| `recycled_pct` | `Option<f64>` | `"recycledPct"` | Recycled content percentage (0–100) |
| `country_of_origin` | `Option<String>` | `"countryOfOrigin"` | ISO 3166-1 alpha-2 country of origin |

### 3.4 Product group vs. product-group sub-classification

`ProductGroup` is the **only** dispatch key: it selects the schema version and the Wasm plugin. `Passport` carries no cross-product group sub-classification field — an earlier `product_category: Option<ProductCategory>` envelope field was removed after measurement found it had zero readers in either this repo or the engine, and every product group that classifies sub-types does so with its own field, under its own name, sourced from its own regulation:

| Product group | Field | Source |
|---|---|---|
| `battery` | `battery_type` | Battery Reg. 2023/1542 Art. 1(3) — closed, five categories, required |
| `steel` | `product_category` | `"flat"` / `"long"` / … |
| `electronics` | `product_category` | `"smartphone"` / `"other-mobile-phone"` / `"cordless-phone"` / `"tablet"` — closed, Reg. (EU) 2023/1670 Art. 1(1) |
| `unsold-goods` | *(none)* | Removed in schema v2.0.0. Impl. Reg. (EU) 2026/2 Art. 3 delimits a disclosure by **CN code**, so its lines carry `cnCategories`, not a category word of ours. See §4.4 |
| `furniture` | `product_type` | — |
| `tyre` | `tyre_class` | `"C1"` / … |

**Rules:**
1. The host dispatches compliance **only** on `ProductGroup`. A plugin is selected by product group, never by a product group-internal field.
2. These fields are plain product group data. A plugin *may* read one to choose an internal rule path, but it does not change which plugin runs.
3. The names and shapes are deliberately uneven — they track what each product group's own act defines, not a normalised cross-product group vocabulary. Only `battery_type` is a closed, required, typed enum; that follows from Art. 1(3) being a named enumeration in law, which is not true of the others.

`Passport::validate()` enforces that `productGroup` matches `productGroupData`'s product group when the latter is present.

### 3.5 Applicable instruments — the law is not on the product group

A product group does **not** determine the law that governs it, and the model no
longer pretends otherwise. `ProductGroupDescriptor` carries identity, scope,
schema versions, disclosure classes and a plugin binding — and no legal fields at
all. Status, legal basis, passport obligation, dates, retention and granularity
are properties of an **(act, product group) pair** and live on `InstrumentBinding`
in the second catalog. See [ARCHITECTURE.md](ARCHITECTURE.md) §`dpp-domain`.

Three records replace what used to be one:

| Record | Answers |
|---|---|
| `Instrument` | *What is this act?* — id, CELEX, `InstrumentKind` (Framework · Delegated · Direct · Adjacent), `InstrumentStatus`, and its `PassportObligation` |
| `ProductGroup­Descriptor` | *What is this group and how do we serve it?* — key, title, schema versions, product categories, disclosure, plugin |
| `InstrumentBinding` | *What does this act do to this group?* — one per pair: status, legal basis, dates, retention, granularity |

**Why a set and not a field.** ESPR **Art. 5(7)** lets one delegated act cover
many product groups and lets a group-specific act supplement a horizontal one,
and the Regulation contains **no precedence rule anywhere** — so overlapping acts
*accumulate*. Applicable instruments are therefore a set, and the governing
requirement is the union.

**Folds are unions, never precedence.** Retention is the **maximum** (periods are
floors). The passport due date is the **earliest** (once the first act's date
arrives, a passport is owed). Granularity is the **most granular** (an item-level
record satisfies a model-level requirement). Provenance folds too: a compound
retention figure is `Sourced` only if *every* contributing figure is.

**"May this bind?" is never folded to a boolean.** `InstrumentCatalog::determinable_for`
returns the (instrument, binding) pairs, not a yes/no, because a determination is
always made *under a named act* — a caller that only learns "yes" cannot say what
it is asserting against. That is exactly how a determination once came to be
emitted against an obligation that did not exist.

**`PassportObligation` is a three-way answer**, not an optional date:
`Required { from }` · `NotRequired` · `DisplacedBy { system, basis }`. The third
is ESPR **Art. 9(4)(b)** — an act whose information duty is discharged through
another system, e.g. EPREL. Without it, an act that creates real, live obligations
but *no passport* could only be recorded as "no date yet", which reads as "a
passport is coming". Determinability and passport duty are **independent
predicates**: ESPR Arts. 24–25 bind today and impose no passport at all.

**Recorded, not computed.** `applicable_instruments` is written at issuance and is
immutable in both senses — it is in `PROTECTED_PATCH_FIELDS` and absent from
`RETENTION_MUTABLE_FIELDS`. Corrections go by supersession. `InstrumentRef` also
carries a `RecordedBasis` of `Catalog` or `Operator`; `Operator` is not a
fallback, it is the case where an act reaches a product whose group no catalog
models and the operator must assert it.

---

## 4. Product group Extensions

Product group-specific data is stored in `ProductGroupData`, a tagged enum. Each variant has its own struct and corresponding JSON schema at `schemas/{product-group}/v{version}.json`.

**Serde**: `ProductGroupData` uses `rename_all = "camelCase"` with internally-tagged format.

### 4.1 Battery Product group (`BatteryData`) — v2.6.0

Source: EU Battery Regulation (EU) 2023/1542. Battery DPP mandatory from
18 Feb 2027.

**This section deliberately does not list every field.** `BatteryData` carries
68, and a hand-maintained copy of that list is the same drift that let four
Annex VI Part A fields sit in the type for seven schema versions with no schema
property to validate against. The authoritative pair is:

- `crates/dpp-domain/src/domain/product group/data/battery.rs` — the type, with a
  per-field regulatory citation on each doc comment.
- `crates/dpp-domain/schemas/battery/v2.6.0.json` — the wire contract, with an
  `x-disclosure` class on every property. `additionalProperties` is `false`, and
  a test asserts the two agree field-for-field.

**Required** (6, and the only ones a passport cannot omit at any category):
`gtin`, `batteryChemistry`, `nominalVoltageV`, `nominalCapacityAh`,
`co2ePerUnitKg`, `batteryType`. Everything else is `Option` — not laxity, but
because the obligations are **per category**: a field mandatory for an
electric-vehicle battery may be "not to be filled/displayed" for an LMT one.
That constraint lives in `dpp_rules::batteries::passport_content`, which the
publish gate enforces, rather than in the schema.

**Shape.** The fields group by the Annex XIII tier that governs who may see
them, which is also what `x-disclosure` records:

| Tier | Annex XIII | Audience | Examples |
|---|---|---|---|
| `public` | point 1, incl. Annex VI Part A via 1(a) | anyone | chemistry, voltages, recycled content, place and date of manufacture |
| `restricted` | point 2 | authorities **and** legitimate interest | cathode/anode/electrolyte composition, dismantling, safety measures |
| `conformity` | point 3 | authorities only | test report results |
| `individual` | point 4 | legitimate interest only | measured performance, state of health, status, use history |

Point 4 is per **item** rather than per model, so those fields nest into
`DynamicPerformance`, `StateOfHealth`, `UsageHistory` and `ExpectedLifetime`
rather than flattening — a declared model figure and a measured one are
different claims and must not sit side by side under near-identical names.

**Legacy fields.** `state_of_health_pct`, `round_trip_efficiency_pct` and
`internal_resistance_mohm` are superseded but retained: a stored record keeps
its value under the name it was written with. See the type's own doc comment
for the rule and for why deletion is reserved for the cases where keeping the
field is itself the defect.

**Helper types**:
- `MaterialComposition { name, weight_pct, cas_number }`
- `CriticalRawMaterial { name, cas_number, weight_grams, country_of_origin }`
- `HazardousSubstance { name, cas_number, concentration_pct }`
- `TemperatureRange { min_c, max_c }`

Schemas: `schemas/battery/v{1.0.0, 2.0.0 … 2.6.0}.json`. Older versions stay
registered so a passport validated against one remains verifiable, and each
carries its own disclosure classes — which is what stops a reclassification
changing the bytes served for an already-published passport.


### 4.2 Textile Product group (`TextileData`) — v1.2.0

Source: ESPR Working Group on Textiles. Delegated act adoption anticipated ~Q2 2027, compliance ~2028–2029.

**Required fields**:

| Field | Rust Type | JSON name |
|---|---|---|
| `fibre_composition` | `Vec<FibreEntry>` | `"fibreComposition"` — must sum to 100% |
| `country_of_origin` | `String` (ISO 3166-1) | `"countryOfOrigin"` |
| `care_instructions` | `String` | `"careInstructions"` — ISO 3758 or plain text |
| `chemical_compliance_standard` | `String` | `"chemicalComplianceStandard"` |

**Optional fields**: `recycled_content_pct`, `carbon_footprint_kg_co2e`, `water_use_litres`, `microplastic_shedding_mg_per_wash`, `repair_score`, `durability_score`, `expected_wash_cycles`, `country_of_raw_material_origin`, `svhc_substances`, `disassembly_instructions`, `spare_parts_available`, `product_weight_grams`.

`FibreEntry { fibre: String, pct: f64, country_of_origin: Option<String> }`

Schemas: `schemas/textile/v1.0.0.json`, `schemas/textile/v1.1.0.json`

### 4.3 The rest of the catalog

Every product group has a typed variant and at least one schema. Only `battery`
and `textile` have models deep enough to warrant their own section above; the
others are listed here rather than each getting a stub.

| Product group | Type | Current schema | Note |
|---|---|---|---|
| `aluminium` | `AluminiumData` | v1.1.0 | Carbon intensity, CBAM-aligned. Intermediate product |
| `construction` | `ConstructionData` | v1.1.0 | CPR (EU) 2024/3110. **Wrong axis** — the CPR defines product *family* → *category* → *type* and uses "product group" zero times. Re-homing needs CPR Annex VII read first |
| `detergent` | `DetergentData` | v1.1.0 | Reg. (EU) 2026/405. Surfactant bands per its Annex VII |
| `electronics` | `ElectronicsData` | v1.2.0 | Narrowed to the four device types Reg. (EU) 2023/1670 Art. 1(1) enumerates. **No passport obligation** — its instruments are EPREL-displaced |
| `furniture` | `FurnitureData` | v1.2.0 | v1.2.0 drops `mattress` from `productType`; v1.1.0 is kept for stored documents |
| `mattress` | `MattressData` | v1.0.0 | Split out of furniture: the working plan makes Mattresses a **separate** product group. Fields are furniture's minus `productType` and **nothing added** — no delegated act exists, so any mattress-specific field would be invented. No Wasm plugin |
| `steel` | `SteelData` | v1.1.0 | CBAM-aligned. Intermediate product, earliest indicative act of any group |
| `toy` | `ToyData` | v1.1.0 | Reg. (EU) 2025/2509 |
| `tyre` | `TyreData` | v1.0.0 | |
| `unsold-goods` | `UnsoldGoodsReport` | v2.0.0 | See §4.4 — **not a product group**; built to Impl. Reg. (EU) 2026/2 Annex I |

`ProductGroupData::Other` keeps the tag and payload of a product group this build
has no typed variant for, verbatim, so an unknown group round-trips rather than
being dropped.

### 4.4 Unsold goods is not a product group, and its model predates its law

ESPR Arts. 24–25 / Annex VII. It occupies a catalog slot for implementation
convenience and borrows textile's plugin, but it is a **horizontal obligation on
an operator over a financial year**, not a product placed on the market. It
carries `PassportObligation::NotRequired`: the duty is real and binding today,
and there is no passport anywhere in Arts. 24–25.

Two acts govern it, both adopted 9 February 2026, and `UnsoldGoodsReport` v2.0.0
is built to them:

- **Commission Implementing Regulation (EU) 2026/2** (CELEX `32026R0002`), under
  Art. 24(3) — Art. 2(1) binds the disclosure to the format in its **Annex I**,
  and Art. 3 delimits categories by **CN code**, first two digits (four for the
  products of its Annex II).
- **Commission Delegated Regulation (EU) 2026/296** (CELEX `32026R0296`), under
  Art. 25(5) — the **closed list of ten derogations**, points (a) to (j). Annex I
  note (h) of 2026/2 makes it the reason vocabulary, so the two interlock.

The shape is Annex I's: an `entity` header (name, EUID-or-other identifier,
standalone vs consolidated with its undertakings listed), a `financialYear` with
both endpoints, a repeating body of `lines`, and the two narrative rows
`measuresTaken` and `measuresPlanned`. Each line carries its CN categories —
plural, because note (f) allows several where items sold together count as one
unit — a description, unit and weight quantities each flagged `estimated` or not,
a packaging-included flag, one `reason`, and a `treatment` split.

Three points that are easy to get wrong:

- **Total destruction is derived, never stored.** Note (i) defines it as
  *recycling + other recovery + disposal*. Preparing-for-reuse and unknown sit
  outside it, which is not the intuitive reading. `WasteTreatmentSplit::total_destruction_pct`
  computes it; the wire has no such field.
- **`unknown` is an answer, not a gap.** Note (i) provides it for the share whose
  treatment could not be obtained from the waste treatment operator, so a
  well-formed split totals exactly 100 and nothing is left over.
- **`CnCategory` is not `CommodityCode`.** Two digits or four, against six/eight/ten
  — a product's own classification is a different level of the same nomenclature
  and files a whole chapter's goods under one article if substituted.

**There is no v1.0.0.** It predated both acts and nothing could carry a document
forward from it: a financial year is not derivable from `"2026-Q2"`, a CN code is
not derivable from `"apparel"`, a six-way split is not derivable from one
destination, and its reason list shares no member with the Art. 2 derogations —
two of its reasons named commercial circumstances that are not derogations at
all. A lens would have had to invent every one of those, so the version was
removed rather than migrated. Safe only because nothing was ever stored under it.

---

## 5. Audit Log (Platform Concern)

Audit logging (who changed what, when, and why) is a platform concern — it is not part of the core domain. The platform layer is responsible for recording state transitions in an append-only audit log (`AuditEntry` records). The core domain enforces lifecycle rules and retention locks but does not define audit storage.

---

## 6. Verifiable Credential Payload

When a DPP transitions to `Published`, it is wrapped in a W3C Verifiable Credential by `dpp-vc` and signed with the operator's Ed25519 key via `dpp-crypto`. The resulting JWS is stored in `jws_signature`.

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://odal-node.io/contexts/dpp/v1"
  ],
  "type": ["VerifiableCredential", "DigitalProductPassport"],
  "issuer": "did:web:manufacturer.example.com",
  "issuanceDate": "2026-04-27T14:32:00Z",
  "credentialSubject": {
    "type": "DigitalProductPassport",
    "schemaVersion": "2.0.0",
    "productName": "EcoCell Pro 48V",
    "manufacturer": { "name": "EcoTech", "address": "DE" },
    "productGroupData": { "batteryChemistry": "LFP", "nominalVoltageV": 48.0 }
  },
  "proof": {
    "type": "JsonWebSignature2020",
    "verificationMethod": "did:web:manufacturer.example.com#key-1",
    "proofPurpose": "assertionMethod",
    "jws": "eyJhbGciOiJFZERTQSJ9..{signature}"
  }
}
```

---

## 7. Schema Versioning

Schemas follow semver. The `VersionedSchemaRegistry` in `dpp-domain` discovers all embedded schemas at compile time and supports runtime registration via `register()` / `register_or_replace()`.

| Version bump | Change type | Backward compatible |
|---|---|---|
| Patch (`1.0.x`) | Clarifications to field descriptions | Yes |
| Minor (`1.x.0`) | New optional fields; provisional -> strict | Yes |
| Major (`x.0.0`) | Field renamed, type changed, or removed | No |

The registered list is `crates/dpp-domain/src/schemas/embedded.rs`, and it is the
only place worth reading for what exists — every product group carries at least
one version and several carry three. Every registered version is reachable at
runtime; a passport is validated against the version it declares, not against
the newest.

No count is written here on purpose. This paragraph used to open "28 embedded
versions across 11 product groups", which was wrong within a day of `mattress`
landing — nine lines below the advice in §"Version bump" that a count is the part
that goes stale while every claim around it stays checkable.
