// ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): All IDTA URNs below must be
// confirmed against the current IDTA catalog (https://industrialdigitaltwin.org/en/content-hub/submodels)
// before release — template IDs and version suffixes change between drafts
// and ratified releases, and an incorrect ID breaks external registry interop.

/// IDTA Nameplate submodel template v2.0 (IDTA 02006-2-0).
/// Used for `ProductIdentification`.
pub const PRODUCT_IDENTIFICATION: &str = "https://admin-shell.io/IDTA/02006/2/0";

/// IDTA Handover Documentation / Manufacturer information (IDTA 02004-1-2).
pub const MANUFACTURER_INFORMATION: &str = "https://admin-shell.io/IDTA/02004/1/2/Manufacturer";

/// IDTA Carbon Footprint submodel template v0.9 (IDTA 02023-0-9).
/// Used for `EnvironmentalImpact` and CO₂e properties.
pub const CARBON_FOOTPRINT: &str = "https://admin-shell.io/IDTA/02023/0/9";

/// IDTA Physical Properties submodel template v1.0 (IDTA 02011-1-0).
/// Used for `MaterialComposition`.
pub const MATERIAL_COMPOSITION: &str = "https://admin-shell.io/IDTA/02011/1/0";

// ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): Confirm this ECLASS IRDI is the
// correct property for the EN 45554 repairability index (not a generic score).
// Verify against the ECLASS Advanced release at https://eclass.eu.
/// ECLASS repairability score property (0173-1#01-AKJ975#001).
pub const REPAIRABILITY: &str = "urn:eclass:0173-1#01-AKJ975#001";

// ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): Confirm Catena-X Battery Pass
// aspect-model version. Check https://github.com/eclipse-tractusx/sldt-semantic-models
// for the current released version tag before PINning.
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

/// Odal Node textile unsold goods report template (EU ESPR Art. 25).
pub const UNSOLD_GOODS_REPORT: &str =
    "urn:odal-node:aas:submodel-template:textile-unsold-goods:1.0";
