//! `dpp-domain` — EU Digital Product Passport domain types and port traits.
//!
//! This crate is the dependency root of the DPP workspace. Every other crate
//! depends on this one. It depends only on `dpp-rules` (pure regulatory rules).
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
pub mod ports;
pub mod schemas;
#[cfg(test)]
mod test_support;

pub use catalog::{
    CatalogError, RegulatoryStatus, RetentionBasis, SectorCatalog, SectorDescriptor,
};

pub use domain::{
    commodity_code::{CommodityCode, CommodityCodeError},
    error::DppError,
    gtin::{Gln, GlnError, Gtin, GtinError, gs1_check_digit},
    identity::{
        Audience, Disclosure, PASSPORT_FIELD_DISCLOSURE, PassportCredential,
        PassportCredentialSubject, SignedCredential,
    },
    lint::{LintFinding, LintResult, LintSeverity, lint_sector_data},
    passport::{
        FacilitySnapshot, ManufacturerInfo, MaterialEntry, Passport, PassportId, PassportView,
    },
    product_identity::ProductIdentity,
    sector::{
        AluminiumData, BatteryChemistry, BatteryData, BatteryStatus, BatteryType, CarbonFootprint,
        CarbonFootprintClass, CarbonFootprintClassError, ConstructionData, DetergentData,
        DynamicPerformance, ElectronicsData, EnergyEfficiencyClass, EnvironmentalReading,
        ExpectedLifetime, FibreEntry, FurnitureData, HarmfulEvents, HazardSymbol,
        HazardousSubstance, LifecycleStage, MaterialComposition, ProductionRoute, RepairCriterion,
        RepairabilityScore, Sector, SectorData, StateOfChargeReading, StateOfHealth, SteelData,
        SurfactantEntry, SvhcSubstance, SystemBoundary, TemperatureRange, TextileData, ToyData,
        TyreData, UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport, UsageHistory,
        redact_sector_data, validate_fibre_composition, validate_surfactants,
        validate_svhc_substances,
    },
    status::PassportStatus,
    transfer::{
        OperatorRole, ResponsibleOperator, TransferChain, TransferError, TransferReason,
        TransferRecord, TransferStatus,
    },
};

pub use domain::field_error::{FieldError, ValidationErrors};

#[cfg(not(target_arch = "wasm32"))]
pub use domain::validation::{
    BatchValidationItem, SectorValidator, SectorValidatorRegistry, batch_errors,
    validate_raw_sector_data, validate_sector_data, validate_sector_data_batch,
    validate_sector_data_with_registry,
};

pub use ports::archive::{
    ArchivePort, ArchiveReceipt, ArchiveStatus, ArchiveVerification, GhostArchive,
};
pub use ports::compliance::{
    ComplianceError, ComplianceErrorKind, ComplianceFinding, ComplianceRegistry, ComplianceResult,
    ComplianceStatus, ComplianceStrategy, gate_determination,
};
pub use ports::registry_sync::{
    GhostRegistrySync, RegistrationRequest, RegistryIdentifiers, RegistryRecord, RegistryStatus,
    RegistrySyncPort,
};

pub use compliance::passthrough_registry::PassthroughRegistry;

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
