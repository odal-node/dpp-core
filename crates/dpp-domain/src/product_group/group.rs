//! [`ProductGroup`] — the EU ESPR dispatch discriminant.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::catalog::ProductGroupCatalog;
use crate::schemas::VersionedSchemaRegistry;

/// The regulated product group a passport belongs to.
///
/// # The enum is an optimisation, not the source of truth
///
/// ProductGroup identity is **data**: the [`ProductGroupCatalog`]
/// manifests, the schema registry and the plugin manifests are all keyed by
/// string. These variants exist so code that genuinely branches per product
/// group — an in-force act with its own typed data — can do so without a
/// string match.
///
/// A product group this build does not know is therefore **not an error**: it
/// deserialises to [`ProductGroup::Other`], which carries the wire tag verbatim so
/// the identity survives a round trip. Adding a product group is a catalog
/// manifest plus a schema, not a release of this crate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProductGroup {
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
    Mattress,
    Detergent,
    /// A product group this build has no typed variant for, holding its wire
    /// tag verbatim. Never a placeholder: the key is preserved, so the
    /// catalog, schema registry and plugin host can all still resolve it.
    Other(String),
}

impl ProductGroup {
    /// Canonical product group key used by the schema registry and the `ProductGroupCatalog`.
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
            Self::Mattress => "mattress",
            Self::Detergent => "detergent",
            Self::Other(key) => key,
        }
    }

    /// The typed variant for a wire tag, or [`ProductGroup::Other`] carrying the tag.
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
            "mattress" => Self::Mattress,
            "detergent" => Self::Detergent,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The serde wire tag for this product group (its `camelCase` JSON discriminant),
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
            Self::Mattress => "mattress",
            Self::Detergent => "detergent",
            Self::Other(tag) => tag,
        }
    }

    /// The values a new passport in this product group may state as its
    /// category: the domain of
    /// [`ProductGroupData::product_category`](crate::product_group::ProductGroupData::product_category),
    /// or `None` where the group has no category axis.
    ///
    /// Read from the `enum` of the group's category property in its **current**
    /// embedded schema, the version every new passport is validated against. So
    /// this is the legal domain, and it is wider than the catalog's
    /// `productCategories`, which is a curated subset: a furniture passport may
    /// lawfully be a `bed`, which the catalog does not list.
    ///
    /// A caller narrowing a credential to categories checks them against this
    /// before issuing it. A value outside it is one no passport can state, so
    /// it admits nothing: a credential narrowed to it alone unlocks no passport,
    /// and that surfaces only when its holder's reads come back public.
    ///
    /// `None` is a real answer, as it is for `product_category`: the group has
    /// no category a passport can state. [`ProductGroup::Other`] always answers
    /// `None`, because nothing in this build knows which of an untyped
    /// payload's keys is a category.
    ///
    /// A passport stored under an earlier schema version is re-validated
    /// against that version, so it may state a value since withdrawn: a
    /// furniture passport from before `v1.2.0` may say `mattress`.
    #[must_use]
    pub fn product_category_values(&self) -> Option<&'static [String]> {
        static VALUES: OnceLock<BTreeMap<&'static str, Vec<String>>> = OnceLock::new();
        if matches!(self, Self::Other(_)) {
            return None;
        }
        VALUES
            .get_or_init(read_category_values)
            .get(self.catalog_key())
            .map(Vec::as_slice)
    }
}

/// Each product group's category axis: the property of its data that
/// [`ProductGroupPayload::product_category`](crate::product_group::ProductGroupPayload::product_category)
/// answers with, by catalog key.
///
/// The correspondence is **not derivable** from the data — depending on product
/// group the category lives under `productCategory`, `productType`,
/// `batteryType`, `productFamily` or `tyreClass` — so it is declared here.
///
/// # A group absent from this table has no category axis
///
/// `mattress`, `textile`, `toy` and `unsold-goods` define none. Impl. Reg. (EU)
/// 2026/2 Art. 3 delimits an unsold-goods disclosure by CN code rather than a
/// category name, so its lines carry `cnCategories` instead.
///
/// `aluminium` is absent, and that is a correction. It used to be listed with
/// `productionRoute`, which certified a **production route** as a product
/// category. The two are different axes: one says how the metal was made, the
/// other what the product is. Pointing a drift guard at the wrong property was
/// worse than having none, because it reported the axis as verified.
const CATEGORY_PROPERTY: &[(&str, &str)] = &[
    ("battery", "batteryType"),
    ("construction", "productFamily"),
    ("detergent", "productType"),
    ("electronics", "productCategory"),
    ("furniture", "productType"),
    ("steel", "productCategory"),
    ("tyre", "tyreClass"),
];

/// Every row of [`CATEGORY_PROPERTY`], resolved against its group's current
/// embedded schema. Built once: the schemas are compiled in, so the answer
/// cannot change while the process runs.
///
/// A row that does not resolve is left out, which reads as "no category axis".
/// That direction fails closed for a caller checking a credential, and
/// `every_product_group_reports_the_category_it_states` keeps it from happening
/// at all: a group that states a category must answer a list holding it.
fn read_category_values() -> BTreeMap<&'static str, Vec<String>> {
    let catalog = ProductGroupCatalog::new();
    let registry = VersionedSchemaRegistry::new();
    CATEGORY_PROPERTY
        .iter()
        .filter_map(|&(key, property)| {
            let version = catalog.current_schema_version(key)?.parse().ok()?;
            let schema: serde_json::Value =
                serde_json::from_str(registry.get(key, &version)?).ok()?;
            let values = schema["properties"][property]["enum"]
                .as_array()?
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect();
            Some((key, values))
        })
        .collect()
}

/// Serialises to the wire tag — including an unknown product group's own tag, so a
/// passport this build cannot type still round-trips byte-identically.
impl Serialize for ProductGroup {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.wire_str())
    }
}

/// Deserialises a known wire tag to its typed variant and **anything else to
/// [`ProductGroup::Other`] carrying that tag**.
///
/// This is the seam that makes the product group axis open. A derived impl would
/// reject an unrecognised tag, which would mean a product group could not
/// exist until this crate was released — the catalog, schemas and plugins are
/// all data, and this is what stops the wire from being the one closed part.
impl<'de> Deserialize<'de> for ProductGroup {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let tag = String::deserialize(deserializer)?;
        Ok(Self::from_wire_tag(&tag))
    }
}
