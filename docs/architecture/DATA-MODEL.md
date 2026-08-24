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

All DPPs — regardless of product group — carry these fields. Source: `dpp-domain/src/domain/passport.rs`.

| Field | Rust Type | JSON name | Description |
|---|---|---|---|
| `id` | `PassportId` (UUID v4) | `"id"` | Unique passport identifier |
| `batch_id` | `Option<String>` | `"batchId"` | Optional batch or lot identifier (ESPR Art. 9) |
| `product_name` | `String` | `"productName"` | Human-readable product name (ESPR Art. 9) |
| `product group` | `ProductGroup` enum | `"product group"` | EU ESPR product group — the **dispatch key** (`battery`, `textile`, …). Selects schema + plugin. |
| `manufacturer` | `ManufacturerInfo` | `"manufacturer"` | Nested: name, address, optional did:web URL |
| `materials` | `Vec<MaterialEntry>` | `"materials"` | Bill of materials entries |
| `co2e_per_unit` | `Option<f64>` | `"co2ePerUnit"` | CO₂e per unit in kg — may be set by compliance engine |
| `repairability_score` | `Option<f64>` | `"repairabilityScore"` | Repairability score (0.0–10.0) |
| `product_group_data` | `Option<ProductGroupData>` | `"productGroupData"` | Typed product-group-specific data (tagged enum) |
| `status` | `PassportStatus` | `"status"` | Lifecycle state (see §2) |
| `qr_code_url` | `Option<String>` | `"qrCodeUrl"` | Public URL for QR code resolution |
| `jws_signature` | `Option<String>` | `"jwsSignature"` | Compact JWS over canonical payload (Ed25519) |
| `created_at` | `DateTime<Utc>` | `"createdAt"` | Record creation timestamp |
| `updated_at` | `DateTime<Utc>` | `"updatedAt"` | Last modification timestamp |
| `published_at` | `Option<DateTime>` | `"publishedAt"` | First publish timestamp |
| `schema_version` | `String` | `"schemaVersion"` | Semver of the product group schema used for validation |
| `retention_locked` | `bool` | `"retentionLocked"` | Set permanently on first publish; prevents deletion |
| `parent_passport_ref` | `Option<PassportRef>` | `"parentPassportRef"` | Cross-operator predecessor this record derives from (second-life lineage). Omitted when absent. |
| `component_refs` | `Vec<PassportRef>` | `"componentRefs"` | Cross-operator references to constituent passports — the bill of materials. Omitted when empty. |

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
| `unsold-goods` | `product_category` | `"apparel"` / `"footwear"` / … |
| `furniture` | `product_type` | — |
| `tyre` | `tyre_class` | `"C1"` / … |

**Rules:**
1. The host dispatches compliance **only** on `ProductGroup`. A plugin is selected by product group, never by a product group-internal field.
2. These fields are plain product group data. A plugin *may* read one to choose an internal rule path, but it does not change which plugin runs.
3. The names and shapes are deliberately uneven — they track what each product group's own act defines, not a normalised cross-product group vocabulary. Only `battery_type` is a closed, required, typed enum; that follows from Art. 1(3) being a named enumeration in law, which is not true of the others.

`Passport::validate()` enforces that `product group` matches `product_group_data`'s product group when the latter is present.

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

### 4.3 Steel Product group — PROVISIONAL

CBAM-aligned. Schema at `schemas/steel/v1.0.0.json`.

### 4.4 Unsold Goods (`UnsoldGoodsReport`)

ESPR Art. 25 / Annex VII destruction-ban reporting for unsold consumer products. Schema at `schemas/unsold-goods/v1.0.0.json`.

### 4.5 Electronics, Other

`ProductGroupData::Electronics` and `ProductGroupData::Other` variants exist but have no product-group-specific struct yet.

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

Current schemas: 28 embedded versions across 11 product groups — see
`crates/dpp-domain/src/schemas/embedded.rs` for the registered list. Every one
is reachable at runtime; a passport is validated against the version it
declares, not against the newest.
