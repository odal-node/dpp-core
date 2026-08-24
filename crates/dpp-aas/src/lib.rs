//! `dpp-aas` — Asset Administration Shell (IDTA) projection of a passport.
//!
//! The IDTA AAS metamodel (IDTA-01001-3-0) is the interoperability standard
//! used by EU Industry 4.0 and Catena-X data spaces. Passports published
//! through Odal Node must be expressible as AAS shells + submodels for
//! ecosystem registry interoperability.
//!
//! Pure, stateless crate with no I/O or network dependencies. Compiles to both
//! `std` and `wasm32`.
//!
//! ## Key types
//!
//! - [`AasShell`] — top-level container; holds [`AssetInformation`] (linking the
//!   shell to the physical product via GS1 GTIN) and references to its submodels.
//! - [`AasSubmodel`] — one logical grouping of product data, e.g.
//!   `ProductIdentification`, `EnvironmentalImpact`, `BatteryTechnicalData`.
//! - [`AasSubmodelElement`] — leaf value ([`AasProperty`]), group
//!   ([`AasCollection`]), or external link ([`AasReference`]).
//!
//! ## Primary entry point
//!
//! [`build_aas_from_passport`] maps a typed [`Passport`](dpp_domain::Passport)
//! plus a GS1 GTIN string into a complete `(AasShell, Vec<AasSubmodel>)` ready
//! for serialisation or registry submission. The shell references its submodels
//! by ID following the IDTA AAS Part 2 API pattern — shell and submodels are
//! served from separate endpoints.
//!
//! Always produces five core submodels: `ProductIdentification`,
//! `ManufacturerInformation`, `EnvironmentalImpact`, `MaterialComposition`,
//! `Repairability`. If `passport.product_group_data` is `Some`, a sixth
//! product group-specific submodel is appended (`BatteryTechnicalData`,
//! `TextileMaterialDeclaration`, `ElectronicsProductData`, or a generic
//! `ProductGroupData` fallback for product groups without a dedicated template yet).
//!
//! The lower-level [`map_dpp_to_aas_submodel`] remains available as a generic
//! JSON-to-submodel escape hatch.
//!
//! # This crate emits identifiers, not conformance
//!
//! Producing AAS-shaped output is not the same as conforming to an IDTA
//! specification, and nothing here should be described as IDTA-conformant.
//!
//! Every `semanticId` emitted is either in the `urn:odal-node:` namespace — our
//! own concept, honestly named — or belongs to a vocabulary the
//! [`dpp-vocab`](../dpp_vocab/index.html) crate records as verified. That rule
//! is enforced by a test, not by a comment, because six identifiers claiming
//! IDTA and ECLASS authority once sat here behind comments asking someone to
//! check them.
//!
//! Split out of [`dpp-digital-link`], which retains the GS1 parser this used to
//! ship alongside.
//!
//! [`dpp-digital-link`]: https://docs.rs/dpp-digital-link

pub mod semantic_ids;

mod builder;
mod mapper;
mod model;
mod product_groups;
mod property;
mod templates;
#[cfg(test)]
mod tests;

pub use builder::{AasError, build_aas_environment, build_aas_from_passport};
pub use mapper::map_dpp_to_aas_submodel;
pub use model::{
    AasCollection, AasDataType, AasEnvironment, AasProperty, AasReference, AasSemId, AasSemIdKey,
    AasShell, AasSubmodel, AasSubmodelElement, AssetInformation, SpecificAssetId,
};
pub use property::{boolean_property, double_property, integer_property, string_property};
pub use templates::{SubmodelTemplate, placeholder_templates, product_group_submodel_template};

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
