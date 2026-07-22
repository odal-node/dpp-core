# Digital Product Passport — Data Model

This document defines the canonical data structures for all Digital Product Passports. It is the authoritative reference for domain types in `dpp-domain`, JSON Schema fields, and Verifiable Credential payloads.

> **Last updated**: 2026-05-29 (aligned to actual `Passport` struct, `BatteryData` v2.0.0, `TextileData` v1.1.0)

---

## 1. Design Principles

1. **Regulation-anchored**: Every mandatory field maps to a specific article in a delegated act or the ESPR framework. Non-regulatory fields are labelled `internal`.
2. **Schema-versioned**: Every passport record carries a `schema_version` field. Migrating to a new regulatory schema does not invalidate old passports.
3. **Sector-extensible**: A base `Passport` struct holds cross-sector fields. Sector-specific data is stored as a typed enum variant (`SectorData`). Adding a new sector requires a new variant and a new JSON schema — no changes to the base type.
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
- `Draft -> Published`: Requires all `strict` fields for the declared sector to be present and valid.
- `Published -> Suspended`: Requires an authenticated action with a stated reason.
- `Suspended -> Published`: Requires re-validation of all `strict` fields.
- `Any -> Archived`: Irreversible.

Custom serde: domain `Published` serialises to wire `"active"` (and back). This matches the EU registry's terminology.

---

## 3. Core Fields

### 3.1 Base Passport (`Passport` struct)

All DPPs — regardless of sector — carry these fields. Source: `dpp-domain/src/domain/passport.rs`.

| Field | Rust Type | JSON name | Description |
|---|---|---|---|
| `id` | `PassportId` (UUID v4) | `"id"` | Unique passport identifier |
| `batch_id` | `Option<String>` | `"batchId"` | Optional batch or lot identifier (ESPR Art. 9) |
| `product_name` | `String` | `"productName"` | Human-readable product name (ESPR Art. 9) |
| `sector` | `Sector` enum | `"sector"` | EU ESPR sector — the **dispatch key** (`battery`, `textile`, …). Selects schema + plugin. |
| `product_category` | `Option<ProductCategory>` | `"productCategory"` | Optional typed sub-type *within* the sector (`smartphone`, `evBattery`…). Not a dispatch key. See §3.5. |
| `manufacturer` | `ManufacturerInfo` | `"manufacturer"` | Nested: name, address, optional did:web URL |
| `materials` | `Vec<MaterialEntry>` | `"materials"` | Bill of materials entries |
| `co2e_per_unit` | `Option<f64>` | `"co2ePerUnit"` | CO₂e per unit in kg — may be set by compliance engine |
| `repairability_score` | `Option<f64>` | `"repairabilityScore"` | Repairability score (0.0–10.0) |
| `sector_data` | `Option<SectorData>` | `"sectorData"` | Typed sector-specific data (tagged enum) |
| `status` | `PassportStatus` | `"status"` | Lifecycle state (see §2) |
| `qr_code_url` | `Option<String>` | `"qrCodeUrl"` | Public URL for QR code resolution |
| `jws_signature` | `Option<String>` | `"jwsSignature"` | Compact JWS over canonical payload (Ed25519) |
| `created_at` | `DateTime<Utc>` | `"createdAt"` | Record creation timestamp |
| `updated_at` | `DateTime<Utc>` | `"updatedAt"` | Last modification timestamp |
| `published_at` | `Option<DateTime>` | `"publishedAt"` | First publish timestamp |
| `schema_version` | `String` | `"schemaVersion"` | Semver of the sector schema used for validation |
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

### 3.4 ProductCategory

Typed `snake_case` enum of sub-types *within* a sector — `EvBattery`, `IndustrialBattery`, `LmtBattery`, `Apparel`, `Footwear`, `HomeTextile`, `Smartphone`, `Laptop`, `Charger`, and `Other(String)` for anything not yet modelled. It is **never** a dispatch key (that is `Sector`); a plugin may branch on it. Carried on `Passport.product_category` as `Option<ProductCategory>`.

> ✅ The former misnomer (a `ProductCategory` enum that actually held *sectors*) was fixed in Phase 2: `Passport` now carries `sector: Sector` (dispatch) and `product_category: Option<ProductCategory>` (this typed sub-type). See §3.5.

### 3.5 Sector vs. Product Category — terminology (IMPORTANT)

These two concepts are routinely confused and **must be kept distinct**. The current model conflates them; this section is the canonical definition and the target shape.

| Concept | What it is | Role | Example values |
|---|---|---|---|
| **Sector** | The EU delegated-act / regulatory domain a product falls under | **Dispatch key** — selects the schema version *and* the Wasm plugin | `battery`, `textile`, `electronics`, `steel` |
| **Product category** | A sub-type *within* a sector | **Data attribute** — a field a sector schema/plugin may branch on internally; never a dispatch key | `ev_battery`, `apparel`, `smartphone`, steel `flat` |

**Rules:**
1. The host dispatches compliance **only** on `Sector`. A plugin is selected by sector, never by product category.
2. Product category is plain sector data. A plugin *may* read it to choose an internal rule path (e.g. battery `portable` vs `ev`), but it does not change which plugin runs.
3. One sector → one plugin → potentially many product categories.

**Realized shape** (✅ implemented in Phase 2 — breaking `x.0.0`):

`Passport` carries **both** `sector: Sector` (dispatch) and `product_category: Option<ProductCategory>` (a typed sub-type). The old misnamed `ProductCategory`-of-sectors enum is gone; `ProductCategory` is now the typed sub-type enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductCategory {
    // Battery
    EvBattery,
    IndustrialBattery,
    LmtBattery,
    // Textile
    Apparel,
    Footwear,
    HomeTextile,
    // Electronics
    Smartphone,
    Laptop,
    Charger,
    // Extensible escape hatch.
    Other(String),
}
```

`Passport::validate()` enforces that `sector` matches `sector_data`'s sector when the latter is present.

**Still open (finer-grained, not part of Phase 2):** sector-*internal* sub-classifications remain as `String` fields on the sector data structs under inconsistent names — `SteelData.product_category` (`"flat"`), `FurnitureData.product_type`, `TyreData.tyre_class` (`"C1"`). These are a different, finer granularity than the top-level `ProductCategory` and can be reconciled later if needed.

---

## 4. Sector Extensions

Sector-specific data is stored in `SectorData`, a tagged enum. Each variant has its own struct and corresponding JSON schema at `schemas/{sector}/v{version}.json`.

**Serde**: `SectorData` uses `rename_all = "camelCase"` with internally-tagged format.

### 4.1 Battery Sector (`BatteryData`) — v2.0.0

Source: EU Battery Regulation (EU) 2023/1542, Annex XIII. Battery DPP mandatory from 18 Feb 2027.

**Required fields** (6):

| Field | Rust Type | JSON name | Reg. Source |
|---|---|---|---|
| `gtin` | `String` (14 digits) | `"gtin"` | GS1 / ESPR |
| `battery_chemistry` | `String` | `"batteryChemistry"` | Art. 13(1)(a) |
| `nominal_voltage_v` | `f64` | `"nominalVoltageV"` | Art. 13(1)(b) |
| `nominal_capacity_ah` | `f64` | `"nominalCapacityAh"` | Art. 13(1)(c) |
| `expected_lifetime_cycles` | `u32` | `"expectedLifetimeCycles"` | Art. 10(1) |
| `co2e_per_unit_kg` | `f64` | `"co2ePerUnitKg"` | Art. 7(1) |

**Optional fields** (21 — all `Option`, `skip_serializing_if = "Option::is_none"`):

| Field | Rust Type | JSON name | Reg. Source |
|---|---|---|---|
| `recycled_content_cobalt_pct` | `Option<f64>` | `"recycledContentCobaltPct"` | Art. 8(1), Annex X |
| `recycled_content_lithium_pct` | `Option<f64>` | `"recycledContentLithiumPct"` | Art. 8(1), Annex X |
| `recycled_content_nickel_pct` | `Option<f64>` | `"recycledContentNickelPct"` | Art. 8(1), Annex X |
| `recycled_content_lead_pct` | `Option<f64>` | `"recycledContentLeadPct"` | Art. 8 (lead-acid) |
| `state_of_health_pct` | `Option<f64>` | `"stateOfHealthPct"` | Art. 14 |
| `rated_capacity_kwh` | `Option<f64>` | `"ratedCapacityKwh"` | Art. 13(1)(d) |
| `rated_energy_wh` | `Option<f64>` | `"ratedEnergyWh"` | Art. 13(1)(d) |
| `carbon_footprint_class` | `Option<String>` | `"carbonFootprintClass"` | Art. 7(2) — A–E |
| `due_diligence_url` | `Option<String>` | `"dueDiligenceUrl"` | Art. 47-52 |
| `cathode_material` | `Option<Vec<MaterialComposition>>` | `"cathodeMaterial"` | Annex XIII §4 |
| `anode_material` | `Option<Vec<MaterialComposition>>` | `"anodeMaterial"` | Annex XIII §4 |
| `electrolyte_material` | `Option<Vec<MaterialComposition>>` | `"electrolyteMaterial"` | Annex XIII §4 |
| `critical_raw_materials` | `Option<Vec<CriticalRawMaterial>>` | `"criticalRawMaterials"` | EU CRM Act 2024/1252 |
| `disassembly_instructions_url` | `Option<String>` | `"disassemblyInstructionsUrl"` | Annex XIII §6 |
| `soh_methodology` | `Option<String>` | `"sohMethodology"` | Art. 14(2) |
| `operating_temp_min_c` | `Option<f64>` | `"operatingTempMinC"` | Annex XIII |
| `operating_temp_max_c` | `Option<f64>` | `"operatingTempMaxC"` | Annex XIII |
| `battery_weight_kg` | `Option<f64>` | `"batteryWeightKg"` | Annex XIII |
| `battery_type` | `Option<String>` | `"batteryType"` | Per regulation: portable, industrial, ev, lmt, starting-lighting-ignition |
| `round_trip_efficiency_pct` | `Option<f64>` | `"roundTripEfficiencyPct"` | Art. 10 — at 50% SoC |
| `internal_resistance_mohm` | `Option<f64>` | `"internalResistanceMohm"` | Art. 10 — at 50% SoC |

**Helper types**:
- `MaterialComposition { name: String, weight_pct: f64, cas_number: Option<String> }`
- `CriticalRawMaterial { name: String, cas_number: Option<String>, weight_grams: Option<f64>, country_of_origin: Option<String> }`

Schema: `schemas/battery/v2.0.0.json`

### 4.2 Textile Sector (`TextileData`) — v1.1.0

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

### 4.3 Steel Sector — PROVISIONAL

CBAM-aligned. Schema at `schemas/steel/v1.0.0.json`.

### 4.4 Unsold Goods (`UnsoldGoodsReport`)

ESPR Art. 25 / Annex VII destruction-ban reporting for unsold consumer products. Schema at `schemas/unsold-goods/v1.0.0.json`.

### 4.5 Electronics, Other

`SectorData::Electronics` and `SectorData::Other` variants exist but have no sector-specific struct yet.

---

## 5. Audit Log (Platform Concern)

Audit logging (who changed what, when, and why) is a platform concern — it is not part of the core domain. The platform layer is responsible for recording state transitions in an append-only audit log (`AuditEntry` records). The core domain enforces lifecycle rules and retention locks but does not define audit storage.

---

## 6. Verifiable Credential Payload

When a DPP transitions to `Published`, it is wrapped in a W3C Verifiable Credential and signed with the operator's Ed25519 key via `dpp-crypto`. The resulting JWS is stored in `jws_signature`.

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
    "productCategory": "BATTERY",
    "productName": "EcoCell Pro 48V",
    "manufacturer": { "name": "EcoTech", "address": "DE" },
    "sectorData": { "batteryChemistry": "LFP", "nominalVoltageV": 48.0 }
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

Current schemas: battery/v2.0.0, textile/v1.0.0, textile/v1.1.0, unsold-goods/v1.0.0, steel/v1.0.0.
