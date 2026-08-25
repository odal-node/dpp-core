//! `dpp-domain` — EU Digital Product Passport domain types and port traits.
//!
//! The dependency root of the DPP workspace: any crate here may depend on it,
//! and it depends only on `dpp-rules` (pure regulatory rules). Not every crate
//! does — `dpp-rules`, `dpp-crypto`, `dpp-calc`, `dpp-vocab`, `dpp-plugin-traits`
//! and `dpp-plugin-sdk` stand on their own, which is why a Wasm product-group
//! plugin never links this crate.
//!
//! No I/O, no async, no HTTP, no database drivers — pure domain logic only.

/// The version of `dpp-core` this build was compiled against.
///
/// All core crates share one version (lockstep), so this crate's version is the
/// workspace's. A consumer cannot otherwise discover it: `CARGO_PKG_VERSION`
/// resolves to the *calling* crate. Platforms embedding this library record it
/// alongside their own version so a compliance determination can be traced to
/// the code that computed it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod access;
pub mod catalog;
pub mod compliance;
pub mod domain;
pub mod error;
pub mod ports;
pub mod schemas;
#[cfg(test)]
mod test_support;
#[cfg(not(target_arch = "wasm32"))]
pub mod validation;

pub use catalog::{
    CatalogError, DateBasis, Granularity, Instrument, InstrumentBinding, InstrumentCatalog,
    InstrumentKind, InstrumentRef, InstrumentStatus, ObligationDate, PassportObligation,
    ProductGroupCatalog, ProductGroupDescriptor, RecordedBasis, RegulatoryStatus, RetentionBasis,
};

pub use domain::{
    commodity_code::{CommodityCode, CommodityCodeError},
    gtin::{Gln, GlnError, Gtin, GtinError, gs1_check_digit},
    identity::{
        Audience, Disclosure, PASSPORT_FIELD_DISCLOSURE, PassportCredential,
        PassportCredentialSubject, SignedCredential,
    },
    lint::{LintFinding, LintResult, LintSeverity, lint_product_group_data},
    passport::{
        FacilitySnapshot, ManufacturerInfo, MaterialEntry, PASSPORT_WIRE_KEYS, Passport,
        PassportId, PassportView, RETENTION_MUTABLE_FIELDS,
    },
    product_group::{
        AluminiumData,
        BatteryChemistry,
        BatteryData,
        BatteryStatus,
        BatteryType,
        CarbonFootprint,
        CarbonFootprintClass,
        CarbonFootprintClassError,
        // The unsold-goods disclosure, whose shape is fixed by Impl. Reg. (EU)
        // 2026/2 Annex I — see `domain::product_group::data::unsold_goods`.
        CnCategory,
        CnCategoryError,
        ConstructionData,
        DetergentData,
        DeviceType,
        DiscardReason,
        DiscardedProductLine,
        DiscardedQuantity,
        DisclosingEntity,
        DisclosureScope,
        DynamicPerformance,
        ElectronicsData,
        EnergyEfficiencyClass,
        EnvironmentalReading,
        ExpectedLifetime,
        FibreEntry,
        FinancialYear,
        FurnitureData,
        HarmfulEvents,
        HazardSymbol,
        HazardousSubstance,
        LegalEntityIdentifier,
        LifecycleStage,
        MaterialComposition,
        MattressData,
        ProductGroup,
        ProductGroupData,
        ProductionRoute,
        RepairCriterion,
        RepairabilityScore,
        StateOfChargeReading,
        StateOfHealth,
        SteelData,
        SurfactantEntry,
        SvhcSubstance,
        SystemBoundary,
        TemperatureRange,
        TextileData,
        ToyData,
        TyreData,
        UnsoldGoodsReport,
        UsageHistory,
        WasteTreatmentSplit,
        redact_product_group_data,
        validate_fibre_composition,
        validate_surfactants,
        validate_svhc_substances,
    },
    product_identity::ProductIdentity,
    status::PassportStatus,
    transfer::{
        OperatorRole, ResponsibleOperator, TransferChain, TransferError, TransferReason,
        TransferRecord, TransferStatus,
    },
};

pub use error::{DppError, FieldError, ValidationErrors};

#[cfg(not(target_arch = "wasm32"))]
pub use validation::{
    BatchValidationItem, ProductGroupValidator, ProductGroupValidatorRegistry, batch_errors,
    validate_passport, validate_product_group_data, validate_product_group_data_batch,
    validate_product_group_data_with_registry, validate_raw_product_group_data,
};

pub use ports::archive::{
    ArchivePort, ArchiveReceipt, ArchiveStatus, ArchiveVerification, GhostArchive,
};
pub use ports::compliance::{
    ComplianceError, ComplianceErrorKind, ComplianceFinding, ComplianceRegistry, ComplianceResult,
    ComplianceStatus, ComplianceStrategy, gate_determination,
};
pub use ports::passport_repo::PROTECTED_PATCH_FIELDS;
pub use ports::registry_sync::{
    GhostRegistrySync, RegistrationRequest, RegistryIdentifiers, RegistryRecord, RegistryStatus,
    RegistrySyncPort,
};

pub use compliance::{PassthroughBatteryStrategy, PassthroughRegistry, PassthroughTextileStrategy};

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
