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
pub mod credential;
pub mod disclosure;
pub mod eol;
pub mod error;
pub mod facility;
pub mod field_error;
pub mod graph;
pub mod identifier;
pub mod instrument;
pub mod lint;
pub mod manufacturer;
pub mod material;
pub mod passport;
pub mod passthrough;
pub mod ports;
pub mod product;
pub mod product_group;
pub mod schemas;
pub mod seal;
pub mod status;
#[cfg(test)]
mod test_support;
pub mod transfer;
pub mod validation;

pub use catalog::{
    CatalogError, Granularity, ProductGroupCatalog, ProductGroupDescriptor, RegulatoryStatus,
    RetentionBasis,
};
pub use instrument::{
    DateBasis, Instrument, InstrumentBinding, InstrumentCatalog, InstrumentKind, InstrumentRef,
    InstrumentStatus, ObligationDate, PassportObligation, RecordedBasis,
};

pub use crate::{
    credential::{PassportCredential, PassportCredentialSubject, SignedCredential},
    disclosure::{Audience, Disclosure, PASSPORT_FIELD_DISCLOSURE},
    identifier::{
        CnCategory, CnCategoryError, CommodityCode, CommodityCodeError, Gln, GlnError, Gtin,
        GtinError, gs1_check_digit,
    },
    lint::{LintFinding, LintResult, LintSeverity, lint_product_group_data},
    passport::{
        FacilitySnapshot, ManufacturerInfo, MaterialEntry, PASSPORT_PROOF_FIELDS,
        PASSPORT_WIRE_KEYS, Passport, PassportId, PassportView, RETENTION_MUTABLE_FIELDS,
    },
    product::ProductIdentity,
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
    },
    status::PassportStatus,
    transfer::{
        OperatorRole, ResponsibleOperator, TransferChain, TransferError, TransferReason,
        TransferRecord, TransferStatus,
    },
};

pub use error::DppError;
pub use field_error::{FieldError, ValidationErrors};

#[cfg(not(target_arch = "wasm32"))]
pub use validation::{
    BatchValidationItem, ProductGroupValidator, ProductGroupValidatorRegistry, batch_errors,
    battery_recycled_chemistry_conflicts, unsold_goods_annex_vii_heading,
    unsold_goods_cn_depth_is_correct, validate_battery_operating_temp, validate_fibre_composition,
    validate_passport, validate_product_group_data, validate_product_group_data_batch,
    validate_product_group_data_with_registry, validate_raw_product_group_data,
    validate_surfactants, validate_svhc_substances,
};

pub use compliance::{
    ComplianceError, ComplianceErrorKind, ComplianceFinding, ComplianceResult, ComplianceStatus,
    gate_determination,
};
pub use ports::archive::{
    ArchivePort, ArchiveReceipt, ArchiveStatus, ArchiveVerification, GhostArchive,
};
pub use ports::compliance::{ComplianceRegistry, ComplianceStrategy};
pub use ports::passport_repo::PROTECTED_PATCH_FIELDS;
pub use ports::registry_sync::{
    GhostRegistrySync, RegistrationRequest, RegistryIdentifiers, RegistryRecord, RegistryStatus,
    RegistrySyncPort,
};

pub use passthrough::{
    PassthroughBatteryStrategy, PassthroughRegistry, PassthroughTextileStrategy,
};

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
