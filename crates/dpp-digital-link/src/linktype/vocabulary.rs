//! GS1 Web Vocabulary link types for Digital Link resolution.

use serde::{Deserialize, Serialize};

/// GS1 defined link types for Digital Link resolution.
///
/// These follow the GS1 Web Vocabulary link type definitions.
/// See: <https://ref.gs1.org/voc/linkTypes>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Gs1LinkType {
    /// `gs1:pip` — Product Information Page (default for consumers).
    ProductInformationPage,
    /// `gs1:epil` — Electronic Product Information Leaflet.
    ElectronicLeaflet,
    /// `gs1:sustainabilityInfo` — Sustainability and environmental data.
    SustainabilityInfo,
    /// `gs1:recyclingInfo` — Recycling instructions and collection points.
    RecyclingInfo,
    /// `gs1:masterData` — Master data about the product (GS1 GDSN).
    MasterData,
    /// `gs1:certificationInfo` — Certification and conformity data.
    CertificationInfo,
    /// `gs1:instructions` — Usage, repair, or disassembly instructions.
    Instructions,
    /// `gs1:safetyInfo` — Safety data sheets, SVHC declarations.
    SafetyInfo,
    /// `gs1:traceability` — Supply chain traceability data.
    Traceability,
    /// `gs1:dpp` — EU Digital Product Passport (full DPP payload).
    /// This is the ESPR-specific link type for accessing the complete DPP.
    DigitalProductPassport,
    /// `odal:predecessor` — the passport this record derives from (cross-operator
    /// second-life successor linkage). Odal-owned vocabulary: the GS1 Web
    /// Vocabulary has no lineage relation.
    Predecessor,
    /// `odal:successor` — the reverse of [`Self::Predecessor`]. Reserved: served
    /// only once a reverse-lineage lookup exists to populate it.
    Successor,
    /// `odal:hasComponent` — a constituent passport in this product's bill of
    /// materials (the assembly points down to a component). Odal-owned vocabulary.
    HasComponent,
    /// `odal:isComponentOf` — the reverse of [`Self::HasComponent`]. Reserved:
    /// served only once a reverse component index exists to populate it.
    IsComponentOf,
    /// Custom / unknown link type (stored as the raw URI).
    Custom(String),
}

impl Gs1LinkType {
    /// Parse a link type from a GS1 vocabulary URI or shorthand.
    pub fn parse(s: &str) -> Self {
        match s {
            "gs1:pip" | "https://ref.gs1.org/voc/pip" => Self::ProductInformationPage,
            "gs1:epil" | "https://ref.gs1.org/voc/epil" => Self::ElectronicLeaflet,
            "gs1:sustainabilityInfo" | "https://ref.gs1.org/voc/sustainabilityInfo" => {
                Self::SustainabilityInfo
            }
            "gs1:recyclingInfo" | "https://ref.gs1.org/voc/recyclingInfo" => Self::RecyclingInfo,
            "gs1:masterData" | "https://ref.gs1.org/voc/masterData" => Self::MasterData,
            "gs1:certificationInfo" | "https://ref.gs1.org/voc/certificationInfo" => {
                Self::CertificationInfo
            }
            "gs1:instructions" | "https://ref.gs1.org/voc/instructions" => Self::Instructions,
            "gs1:safetyInfo" | "https://ref.gs1.org/voc/safetyInfo" => Self::SafetyInfo,
            "gs1:traceability" | "https://ref.gs1.org/voc/traceability" => Self::Traceability,
            "gs1:dpp" | "https://ref.gs1.org/voc/dpp" => Self::DigitalProductPassport,
            "odal:predecessor" | "https://ref.odal-node.io/voc/predecessor" => Self::Predecessor,
            "odal:successor" | "https://ref.odal-node.io/voc/successor" => Self::Successor,
            "odal:hasComponent" | "https://ref.odal-node.io/voc/hasComponent" => Self::HasComponent,
            "odal:isComponentOf" | "https://ref.odal-node.io/voc/isComponentOf" => {
                Self::IsComponentOf
            }
            other => Self::Custom(other.to_owned()),
        }
    }

    /// Return the canonical GS1 vocabulary URI for this link type.
    pub fn as_gs1_uri(&self) -> &str {
        match self {
            Self::ProductInformationPage => "https://ref.gs1.org/voc/pip",
            Self::ElectronicLeaflet => "https://ref.gs1.org/voc/epil",
            Self::SustainabilityInfo => "https://ref.gs1.org/voc/sustainabilityInfo",
            Self::RecyclingInfo => "https://ref.gs1.org/voc/recyclingInfo",
            Self::MasterData => "https://ref.gs1.org/voc/masterData",
            Self::CertificationInfo => "https://ref.gs1.org/voc/certificationInfo",
            Self::Instructions => "https://ref.gs1.org/voc/instructions",
            Self::SafetyInfo => "https://ref.gs1.org/voc/safetyInfo",
            Self::Traceability => "https://ref.gs1.org/voc/traceability",
            Self::DigitalProductPassport => "https://ref.gs1.org/voc/dpp",
            Self::Predecessor => "https://ref.odal-node.io/voc/predecessor",
            Self::Successor => "https://ref.odal-node.io/voc/successor",
            Self::HasComponent => "https://ref.odal-node.io/voc/hasComponent",
            Self::IsComponentOf => "https://ref.odal-node.io/voc/isComponentOf",
            Self::Custom(uri) => uri.as_str(),
        }
    }
}

#[cfg(test)]
mod tests;
