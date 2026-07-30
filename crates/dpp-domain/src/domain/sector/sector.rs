//! [`Sector`] — the EU ESPR dispatch discriminant.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The regulated product group a passport belongs to.
///
/// # The enum is an optimisation, not the source of truth
///
/// Sector identity is **data**: the [`SectorCatalog`](crate::catalog::SectorCatalog)
/// manifests, the schema registry and the plugin manifests are all keyed by
/// string. These variants exist so code that genuinely branches per product
/// group — an in-force act with its own typed data — can do so without a
/// string match.
///
/// A sector this build does not know is therefore **not an error**: it
/// deserialises to [`Sector::Other`], which carries the wire tag verbatim so
/// the identity survives a round trip. Adding a product group is a catalog
/// manifest plus a schema, not a release of this crate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Sector {
    Battery,
    Textile,
    UnsoldGoods,
    Steel,
    Electronics,
    Construction,
    Tyre,
    Toy,
    Aluminium,
    Furniture,
    Detergent,
    /// A product group this build has no typed variant for, holding its wire
    /// tag verbatim. Never a placeholder: the key is preserved, so the
    /// catalog, schema registry and plugin host can all still resolve it.
    Other(String),
}

impl Sector {
    /// Canonical sector key used by the schema registry and the `SectorCatalog`.
    ///
    /// This is the one true spelling (kebab-case where needed), distinct from
    /// the enum's camelCase serde tag — e.g. `UnsoldGoods` serialises as
    /// `"unsoldGoods"` but its catalog/registry key is `"unsold-goods"`.
    pub fn catalog_key(&self) -> &str {
        match self {
            Self::Battery => "battery",
            Self::Textile => "textile",
            Self::UnsoldGoods => "unsold-goods",
            Self::Steel => "steel",
            Self::Electronics => "electronics",
            Self::Construction => "construction",
            Self::Tyre => "tyre",
            Self::Toy => "toy",
            Self::Aluminium => "aluminium",
            Self::Furniture => "furniture",
            Self::Detergent => "detergent",
            Self::Other(key) => key,
        }
    }

    /// The typed variant for a wire tag, or [`Sector::Other`] carrying the tag.
    ///
    /// Accepts the catalog's kebab-case spelling as well as the camelCase wire
    /// tag, because both are in circulation — `unsold-goods` and `unsoldGoods`
    /// name the same product group.
    #[must_use]
    pub fn from_wire_tag(tag: &str) -> Self {
        match tag {
            "battery" => Self::Battery,
            "textile" => Self::Textile,
            "unsoldGoods" | "unsold-goods" => Self::UnsoldGoods,
            "steel" => Self::Steel,
            "electronics" => Self::Electronics,
            "construction" => Self::Construction,
            "tyre" => Self::Tyre,
            "toy" => Self::Toy,
            "aluminium" => Self::Aluminium,
            "furniture" => Self::Furniture,
            "detergent" => Self::Detergent,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The serde wire tag for this sector (its `camelCase` JSON discriminant),
    /// e.g. `"unsoldGoods"` — distinct from [`Self::catalog_key`], which is
    /// kebab-case (`"unsold-goods"`) and used by the schema registry/catalog.
    ///
    /// Equivalent to `serde_json::to_value(self)` but without the allocation
    /// and `Value` round trip.
    pub fn wire_str(&self) -> &str {
        match self {
            Self::Battery => "battery",
            Self::Textile => "textile",
            Self::UnsoldGoods => "unsoldGoods",
            Self::Steel => "steel",
            Self::Electronics => "electronics",
            Self::Construction => "construction",
            Self::Tyre => "tyre",
            Self::Toy => "toy",
            Self::Aluminium => "aluminium",
            Self::Furniture => "furniture",
            Self::Detergent => "detergent",
            Self::Other(tag) => tag,
        }
    }
}

/// Serialises to the wire tag — including an unknown sector's own tag, so a
/// passport this build cannot type still round-trips byte-identically.
impl Serialize for Sector {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.wire_str())
    }
}

/// Deserialises a known wire tag to its typed variant and **anything else to
/// [`Sector::Other`] carrying that tag**.
///
/// This is the seam that makes the sector axis open. A derived impl would
/// reject an unrecognised tag, which would mean a product group could not
/// exist until this crate was released — the catalog, schemas and plugins are
/// all data, and this is what stops the wire from being the one closed part.
impl<'de> Deserialize<'de> for Sector {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let tag = String::deserialize(deserializer)?;
        Ok(Self::from_wire_tag(&tag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_str_matches_serde_serialization() {
        for sector in [
            Sector::Battery,
            Sector::Textile,
            Sector::UnsoldGoods,
            Sector::Steel,
            Sector::Electronics,
            Sector::Construction,
            Sector::Tyre,
            Sector::Toy,
            Sector::Aluminium,
            Sector::Furniture,
            Sector::Detergent,
            Sector::Other("packaging".into()),
        ] {
            let serialized = serde_json::to_value(&sector).unwrap();
            assert_eq!(
                serialized.as_str().unwrap(),
                sector.wire_str(),
                "wire_str() disagrees with serde for {sector:?}"
            );
        }
    }
}
