//! `dpp-digital-link` — GS1 Digital Link parsing, AAS mapping, JSON-LD, and link-type
//! negotiation.
//!
//! Pure, stateless crate with no I/O or network dependencies. Compiles to both
//! `std` and `wasm32`.

pub mod aas;
pub mod digital_link;
pub mod jsonld;
pub mod linktype;

pub use aas::{
    AasCollection, AasDataType, AasProperty, AasReference, AasSemId, AasSemIdKey, AasShell,
    AasSubmodel, AasSubmodelElement, AasSubmodelRef, AssetInformation, SpecificAssetId,
    SubmodelTemplate, boolean_property, build_aas_from_passport, double_property, integer_property,
    map_dpp_to_aas_submodel, placeholder_templates, sector_submodel_template, semantic_ids,
    string_property,
};
pub use digital_link::{
    AI_TABLE, AiDescriptor, AiRole, DigitalLink, DigitalLinkError, ai_descriptor, build_qr_url,
    short_serial, validate_gtin,
};
pub use linktype::{
    AccessTier, DppMediaType, Gs1LinkType, LinkDescriptor, ResolutionRequest, negotiate,
};
