// Every identifier below is either in our own `urn:odal-node:` namespace or is
// a third-party identifier that was verified against its authority's own
// published source. There is deliberately no third category.
//
// Five identifiers claiming IDTA and ECLASS authority were removed on
// 2026-07-29 because they were wrong — not stale, wrong. IDTA semanticIds are
// name-based (`https://admin-shell.io/idta/nameplate/3/0/Nameplate`,
// `.../idta/CarbonFootprint/CarbonFootprint/0/9`); the removed ones used a
// `admin-shell.io/IDTA/<specification-document-number>/<v>/<v>` form that IDTA
// does not use, so no consumer would have resolved them. The ECLASS IRDI
// additionally carried Code Space Identifier `01` (Classification Class) while
// being used as a Property semanticId, where `02` is the Property space.
//
// Adopting the *correct* IDTA identifiers is a separate change and needs the
// specification PDFs read first. Coining an honest `urn:odal-node:` identifier
// is not a lesser option: it says "this is our concept" truthfully, where a
// wrong IRDI says "this is IDTA's concept" falsely, to a machine.

// ── Submodel templates ───────────────────────────────────────────────────────
// The semanticId of a Submodel. Distinct from the properties inside it: an AAS
// Submodel and an AAS Property are different concepts, and one URI cannot
// honestly denote both.

/// Odal Node product identification template.
pub const PRODUCT_IDENTIFICATION: &str =
    "urn:odal-node:aas:submodel-template:product-identification:1.0";

/// Odal Node manufacturer information template.
pub const MANUFACTURER_INFORMATION: &str =
    "urn:odal-node:aas:submodel-template:manufacturer-information:1.0";

/// Odal Node environmental impact template.
/// Will be replaced by IDTA's Carbon Footprint template once its semanticId is
/// read from the specification (IDTA 02023, v1.0.1 at time of writing).
pub const CARBON_FOOTPRINT: &str = "urn:odal-node:aas:submodel-template:carbon-footprint:1.0";

/// Odal Node material composition template.
pub const MATERIAL_COMPOSITION: &str =
    "urn:odal-node:aas:submodel-template:material-composition:1.0";

/// Odal Node repairability template.
pub const REPAIRABILITY: &str = "urn:odal-node:aas:submodel-template:repairability:1.0";

/// Odal Node generic passport template, for payloads mapped without a typed
/// sector builder.
pub const DIGITAL_PRODUCT_PASSPORT: &str =
    "urn:odal-node:aas:submodel-template:digital-product-passport:1.0";

// ── Properties ───────────────────────────────────────────────────────────────
// The semanticId of an individual SubmodelElement.

/// Product name property.
pub const PRODUCT_NAME: &str = "urn:odal-node:aas:property:product-name:1.0";

/// Manufacturer name property.
pub const MANUFACTURER_NAME: &str = "urn:odal-node:aas:property:manufacturer-name:1.0";

/// Cradle-to-gate CO₂e per unit property, in kg CO₂e.
pub const CO2E_PER_UNIT: &str = "urn:odal-node:aas:property:co2e-per-unit:1.0";

/// Overall repairability index property (EN 45554 scale).
pub const REPAIRABILITY_SCORE: &str = "urn:odal-node:aas:property:repairability-score:1.0";

// ✅ Verified 2026-07-29 against https://github.com/eclipse-tractusx/sldt-semantic-models
// — `io.catenax.battery.battery_pass/6.0.0` is a real released aspect model and
// 6.0.0 is current. The only third-party identifier in this file that survived
// that day's audit.
/// Catena-X Battery Pass submodel template v6.0.0.
pub const BATTERY_TECHNICAL_DATA: &str =
    "urn:samm:io.catenax.battery.battery_pass:6.0.0#BatteryPass";

/// Odal Node textile material declaration template.
/// Will be replaced when IDTA publishes an official textile submodel template.
pub const TEXTILE_MATERIAL: &str = "urn:odal-node:aas:submodel-template:textile-material:1.0";

/// Odal Node electronics product data template.
/// Will be replaced when IDTA publishes an official electronics submodel template.
pub const ELECTRONICS_PRODUCT_DATA: &str =
    "urn:odal-node:aas:submodel-template:electronics-product-data:1.0";

// ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): All odal-node URNs below are
// placeholders. Replace with official IDTA URNs when templates are published.

/// Odal Node steel product data template (EU ESPR carbon intensity).
pub const STEEL_PRODUCT_DATA: &str = "urn:odal-node:aas:submodel-template:steel-product-data:1.0";

/// Odal Node construction product data template (EU CPR 2024/3110).
pub const CONSTRUCTION_PRODUCT_DATA: &str =
    "urn:odal-node:aas:submodel-template:construction-product-data:1.0";

/// Odal Node tyre product data template (EU Regulation 2020/740).
pub const TYRE_PRODUCT_DATA: &str = "urn:odal-node:aas:submodel-template:tyre-product-data:1.0";

/// Odal Node toy product data template (EU 2025/2509).
pub const TOY_PRODUCT_DATA: &str = "urn:odal-node:aas:submodel-template:toy-product-data:1.0";

/// Odal Node aluminium product data template (EU ESPR ~2030, CBAM-aligned).
pub const ALUMINIUM_PRODUCT_DATA: &str =
    "urn:odal-node:aas:submodel-template:aluminium-product-data:1.0";

/// Odal Node furniture product data template (EU ESPR ~2028-2031).
pub const FURNITURE_PRODUCT_DATA: &str =
    "urn:odal-node:aas:submodel-template:furniture-product-data:1.0";

/// Odal Node detergent product data template (EU 2026/405).
pub const DETERGENT_PRODUCT_DATA: &str =
    "urn:odal-node:aas:submodel-template:detergent-product-data:1.0";

/// Odal Node unsold goods report template (EU ESPR Art. 25).
pub const UNSOLD_GOODS_REPORT: &str = "urn:odal-node:aas:submodel-template:unsold-goods:1.0";
