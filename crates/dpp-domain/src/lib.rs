//! `dpp-domain` — EU Digital Product Passport domain types and port traits.
//!
//! This crate is the dependency root of the DPP workspace. Every other crate
//! depends on this one. It depends only on `dpp-rules` (pure regulatory rules).
//!
//! No I/O, no async, no HTTP, no database drivers — pure domain logic only.

pub mod catalog;
pub mod compliance;
pub mod domain;
pub mod ports;
pub mod schemas;

pub use catalog::{CatalogError, RegulatoryStatus, SectorCatalog, SectorDescriptor};

pub use domain::{
    error::DppError,
    gtin::{Gln, GlnError, Gtin, GtinError, gs1_check_digit},
    identity::{AccessTier, PassportCredential, PassportCredentialSubject, SignedCredential},
    passport::{
        ManufacturerInfo, MaterialEntry, Passport, PassportId, PassportView, ProductCategory,
    },
    sector::{
        AluminiumData, BatteryChemistry, BatteryData, BatteryType, CarbonFootprint,
        CarbonFootprintClass, ConstructionData, DetergentData, ElectronicsData,
        EnergyEfficiencyClass, FibreEntry, FurnitureData, LifecycleStage, MaterialComposition,
        ProductionRoute, RepairCriterion, RepairabilityScore, Sector, SectorData, SteelData,
        SurfactantEntry, SvhcSubstance, SystemBoundary, TextileData, ToyData, TyreData,
        UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport, redact_sector_data,
        validate_fibre_composition, validate_surfactants, validate_svhc_substances,
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
    ComplianceError, ComplianceErrorKind, ComplianceRegistry, ComplianceResult, ComplianceStatus,
    ComplianceStrategy, gate_determination,
};
pub use ports::registry_sync::{
    GhostRegistrySync, RegistrationRequest, RegistryIdentifiers, RegistryRecord, RegistryStatus,
    RegistrySyncPort,
};

pub use compliance::passthrough_registry::PassthroughRegistry;
