//! The [`ProductGroupData`] discriminated union and its per-audience redaction.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::aluminium::AluminiumData;
use super::battery::BatteryData;
use super::construction::ConstructionData;
use super::detergent::DetergentData;
use super::electronics::ElectronicsData;
use super::furniture::FurnitureData;
use super::mattress::MattressData;
use super::steel::SteelData;
use super::textile::TextileData;
use super::toy::ToyData;
use super::tyre::TyreData;
use super::unsold_goods::UnsoldGoodsReport;
use crate::domain::product_group::ProductGroup;

/// Typed, product group-specific DPP data — replaces the opaque `compliance_data: Value`.
///
/// Serialises as an internally-tagged object where `"productGroup"` is the
/// discriminant field, e.g.:
/// ```json
/// { "productGroup": "battery", "gtin": "09506000134352", "nominalVoltageV": 3.2, ... }
/// ```
/// ```json
/// { "productGroup": "textile", "fibreComposition": [...], "countryOfOrigin": "BD" }
/// ```
/// An unknown `product_group` tag deserialises to [`ProductGroupData::Other`], which keeps
/// both the tag and the payload verbatim — see the hand-written `Deserialize`
/// below for why the derive cannot do this.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ProductGroupData {
    /// Boxed, unlike its siblings. Battery carries more law than any other
    /// product group — the Annex XIII point 1 and point 4 tiers are most of
    /// `BatteryData` — and an unboxed variant makes *every* `ProductGroupData`, a
    /// toy's included, as large as the largest one. Boxing keeps the enum the
    /// size of its second-largest variant and costs one indirection on the
    /// battery path alone. Serde treats `Box<T>` as `T`, so the wire is
    /// unchanged.
    Battery(Box<BatteryData>),
    /// Boxed for the same reason as [`ProductGroupData::Battery`]: textile is the
    /// other product group with a real model rather than a stub, and leaving it
    /// inline would keep every variant the size of this one. The rule is the
    /// payload's size, not the product group — a stub that grows a body gets boxed
    /// when it does.
    Textile(Box<TextileData>),
    UnsoldGoods(UnsoldGoodsReport),
    Steel(SteelData),
    Electronics(ElectronicsData),
    Construction(ConstructionData),
    Tyre(TyreData),
    Toy(ToyData),
    Aluminium(AluminiumData),
    Furniture(FurnitureData),
    Mattress(MattressData),
    Detergent(DetergentData),
    /// A product group this build has no typed variant for.
    ///
    /// `product_group` is the wire tag verbatim and `data` is the whole object. A
    /// passport for a product group added to the catalog after this crate was
    /// released survives a round trip unchanged — which is the property that
    /// makes adding a product group a data change rather than a release.
    Other {
        /// The wire tag exactly as received.
        product_group: String,
        /// The full object, including its `product_group` key.
        data: serde_json::Value,
    },
}

// ─── Wire format ─────────────────────────────────────────────────────────────
//
// Hand-written because the product group tag is open. `#[serde(tag = "productGroup")]`
// enumerates its variants at compile time and rejects anything else, which
// would make the wire the one closed part of an otherwise data-driven product group
// model: the catalog, the schema registry and the plugin manifests are all
// keyed by string, so a product group added to the catalog would still fail to
// deserialize until this crate was released.
//
// The shape is unchanged — an internally-tagged object with `product_group` as the
// discriminant. Only the unknown-tag behaviour differs: fall through to
// `Other`, keeping tag and payload, instead of failing.

impl Serialize for ProductGroupData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Serialise the payload, then stamp the tag onto it. Going through
        // `Value` keeps one definition of the tag rather than repeating the
        // struct field list per variant.
        let mut value = match self {
            Self::Battery(d) => serde_json::to_value(d),
            Self::Textile(d) => serde_json::to_value(d),
            Self::UnsoldGoods(d) => serde_json::to_value(d),
            Self::Steel(d) => serde_json::to_value(d),
            Self::Electronics(d) => serde_json::to_value(d),
            Self::Construction(d) => serde_json::to_value(d),
            Self::Tyre(d) => serde_json::to_value(d),
            Self::Toy(d) => serde_json::to_value(d),
            Self::Aluminium(d) => serde_json::to_value(d),
            Self::Furniture(d) => serde_json::to_value(d),
            Self::Mattress(d) => serde_json::to_value(d),
            Self::Detergent(d) => serde_json::to_value(d),
            Self::Other { data, .. } => Ok(data.clone()),
        }
        .map_err(serde::ser::Error::custom)?;

        let tag = self.product_group();
        if let serde_json::Value::Object(ref mut map) = value {
            map.insert(
                "productGroup".to_owned(),
                serde_json::Value::String(tag.wire_str().to_owned()),
            );
        }
        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProductGroupData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;

        let value = serde_json::Value::deserialize(deserializer)?;
        let tag = value
            .get("productGroup")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| D::Error::missing_field("productGroup"))?
            .to_owned();

        // A typed variant ignores the `product_group` key it does not declare, so the
        // whole object can be handed to it unchanged.
        macro_rules! typed {
            ($variant:ident) => {
                serde_json::from_value(value.clone())
                    .map(Self::$variant)
                    .map_err(D::Error::custom)
            };
        }

        match ProductGroup::from_wire_tag(&tag) {
            ProductGroup::Battery => typed!(Battery),
            ProductGroup::Textile => typed!(Textile),
            ProductGroup::UnsoldGoods => typed!(UnsoldGoods),
            ProductGroup::Steel => typed!(Steel),
            ProductGroup::Electronics => typed!(Electronics),
            ProductGroup::Construction => typed!(Construction),
            ProductGroup::Tyre => typed!(Tyre),
            ProductGroup::Toy => typed!(Toy),
            ProductGroup::Aluminium => typed!(Aluminium),
            ProductGroup::Furniture => typed!(Furniture),
            ProductGroup::Mattress => typed!(Mattress),
            ProductGroup::Detergent => typed!(Detergent),
            ProductGroup::Other(product_group) => Ok(Self::Other {
                product_group,
                data: value,
            }),
        }
    }
}

impl ProductGroupData {
    /// Build an untyped payload, reading the tag from the object's own
    /// `product_group` field. Falls back to `"other"` when the object carries none.
    ///
    /// Returns `None` if `data` is not a JSON object, or if the tag names a
    /// product group this build *does* type.
    ///
    /// # Why this can fail
    ///
    /// `Other` holding a typed product group's tag would be a second representation of
    /// that product group which does not compare equal to the first: `Other("battery")`
    /// is not `ProductGroup::Battery` under `Eq` or `Hash`, would miss every typed
    /// match arm, and would be refused by `validate_product_group_data` even though the
    /// same bytes deserialize into a valid `Battery`. Deserialization already
    /// routes known tags to their variants; this constructor must not offer a
    /// way around it.
    ///
    /// **A non-object payload is refused for a different and sharper reason.**
    /// [`Serialize`] stamps the `product_group` tag onto the payload only when it is an
    /// object, so an array or scalar serialises with no tag at all. Two things
    /// follow, and the second is the one that matters:
    ///
    /// 1. It does not round-trip — [`Deserialize`] requires a `product_group` key and
    ///    rejects what this produced.
    /// 2. `dpp-aas`'s unknown-product group backstop keys off that tag. With no tag it
    ///    returns early, leaving the payload to a policy whose default class is
    ///    `Public` — so an untagged array's contents would be served to every
    ///    audience.
    ///
    /// Only a Rust caller could construct one: deserialization cannot, since it
    /// needs the object to find the tag in the first place. Refusing here closes
    /// it at the only door it has.
    #[must_use]
    pub fn other(mut data: serde_json::Value) -> Option<Self> {
        let map = data.as_object_mut()?;
        let product_group = map
            .get("productGroup")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("other")
            .to_owned();
        // `Other::data` is documented as the full object *including* its
        // `product_group` key, and `Deserialize` builds it that way. An untagged input
        // would otherwise produce a value that serialises with the tag and
        // deserialises back unequal to itself — the same value, two shapes,
        // depending on whether it had been through the wire yet.
        map.entry("productGroup")
            .or_insert_with(|| serde_json::Value::String(product_group.clone()));
        match ProductGroup::from_wire_tag(&product_group) {
            ProductGroup::Other(_) => Some(Self::Other {
                product_group,
                data,
            }),
            _ => None,
        }
    }

    /// Returns the `ProductGroup` discriminant for this data.
    pub fn product_group(&self) -> ProductGroup {
        match self {
            ProductGroupData::Battery(_) => ProductGroup::Battery,
            ProductGroupData::Textile(_) => ProductGroup::Textile,
            ProductGroupData::UnsoldGoods(_) => ProductGroup::UnsoldGoods,
            ProductGroupData::Steel(_) => ProductGroup::Steel,
            ProductGroupData::Electronics(_) => ProductGroup::Electronics,
            ProductGroupData::Construction(_) => ProductGroup::Construction,
            ProductGroupData::Tyre(_) => ProductGroup::Tyre,
            ProductGroupData::Toy(_) => ProductGroup::Toy,
            ProductGroupData::Aluminium(_) => ProductGroup::Aluminium,
            ProductGroupData::Furniture(_) => ProductGroup::Furniture,
            ProductGroupData::Mattress(_) => ProductGroup::Mattress,
            ProductGroupData::Detergent(_) => ProductGroup::Detergent,
            ProductGroupData::Other { product_group, .. } => {
                ProductGroup::Other(product_group.clone())
            }
        }
    }

    /// The product **model** identifier this product group's data carries, if any.
    ///
    /// A registry registration made at item level must link the model
    /// identifier where a model design exists for the product, and a batch-level
    /// one must do the same — so a caller building a registration needs to know
    /// which field, for this product group, *is* the model identifier. That is
    /// domain knowledge about the product group, which is why it lives here
    /// rather than in whatever assembles the registration.
    ///
    /// `None` is a substantive answer, not a fallback: it says this product
    /// group models no model identifier, which is the lawful state for products
    /// that have no model design. It must never be returned merely because a
    /// product group was overlooked — hence the exhaustive match.
    ///
    /// [`ProductGroupData::Other`] always returns `None`: the payload is an untyped
    /// object for a product group this build has no variant for, so nothing here
    /// can say which of its keys is a model identifier.
    #[must_use]
    pub fn model_identifier(&self) -> Option<&str> {
        match self {
            // Annex XIII §1 — the manufacturer's battery model identifier, as it
            // appears on the label or in the technical documentation.
            ProductGroupData::Battery(d) => d.battery_model_id.as_deref(),
            // No other product group models a model identifier yet. Listed
            // rather than wildcarded so adding a product group that does is a compile
            // error here, not a silent "no model design exists" told to a
            // registry.
            ProductGroupData::Textile(_)
            | ProductGroupData::UnsoldGoods(_)
            | ProductGroupData::Steel(_)
            | ProductGroupData::Electronics(_)
            | ProductGroupData::Construction(_)
            | ProductGroupData::Tyre(_)
            | ProductGroupData::Toy(_)
            | ProductGroupData::Aluminium(_)
            | ProductGroupData::Furniture(_)
            | ProductGroupData::Mattress(_)
            | ProductGroupData::Detergent(_)
            | ProductGroupData::Other { .. } => None,
        }
    }

    /// The GTIN carried by this product group's typed data, if any.
    ///
    /// `UnsoldGoods` and `Other` carry no GTIN field — a discard-event report
    /// and an untyped catch-all respectively, neither of which identifies a
    /// trade item the way every other product group does.
    pub fn gtin(&self) -> Option<&str> {
        match self {
            ProductGroupData::Battery(d) => Some(d.gtin.as_str()),
            ProductGroupData::Textile(d) => Some(d.gtin.as_str()),
            ProductGroupData::Steel(d) => Some(d.gtin.as_str()),
            ProductGroupData::Electronics(d) => Some(d.gtin.as_str()),
            ProductGroupData::Construction(d) => Some(d.gtin.as_str()),
            ProductGroupData::Tyre(d) => Some(d.gtin.as_str()),
            ProductGroupData::Toy(d) => Some(d.gtin.as_str()),
            ProductGroupData::Aluminium(d) => Some(d.gtin.as_str()),
            ProductGroupData::Furniture(d) => Some(d.gtin.as_str()),
            ProductGroupData::Mattress(d) => Some(d.gtin.as_str()),
            ProductGroupData::Detergent(d) => Some(d.gtin.as_str()),
            ProductGroupData::UnsoldGoods(_) | ProductGroupData::Other { .. } => None,
        }
    }
}

/// Serialize `data` to a JSON object and strip any top-level field the
/// `audience` may not see.
///
/// `descriptor.disclosure` maps camelCase JSON field names to their
/// [`Disclosure`](crate::domain::identity::Disclosure) class; visibility is
/// decided by [`Audience::may_see`](crate::domain::identity::Audience::may_see),
/// which is a lattice and not a threshold. Fields not listed in the map are
/// always retained (default: Public).
///
/// Returns a `serde_json::Value::Object` with redacted fields removed.
/// Returns `serde_json::Value::Null` if serialization fails.
pub fn redact_product_group_data(
    data: &ProductGroupData,
    audience: crate::domain::identity::Audience,
    descriptor: &crate::catalog::ProductGroupDescriptor,
) -> serde_json::Value {
    let mut value = match serde_json::to_value(data) {
        Ok(v) => v,
        Err(_) => return serde_json::Value::Null,
    };
    if let Some(obj) = value.as_object_mut() {
        obj.retain(|key, _| match descriptor.disclosure.get(key) {
            Some(&class) => audience.may_see(class),
            None => true,
        });
    }
    value
}

#[cfg(test)]
mod other_constructor_tests {
    use super::*;
    use serde_json::json;

    /// A non-object payload is refused rather than accepted untagged.
    ///
    /// The hazard is not the round-trip failure, which is merely wrong. It is
    /// that `Serialize` only stamps the `product_group` tag onto an object, so an
    /// untagged payload slips past `dpp-aas`'s unknown-product group backstop — which
    /// keys off exactly that tag — and is filtered by a policy defaulting to
    /// `Public`.
    #[test]
    fn a_non_object_payload_is_refused() {
        for payload in [
            json!([{ "secret": "value" }]),
            json!(42),
            json!("battery"),
            json!(null),
            json!(true),
        ] {
            assert!(
                ProductGroupData::other(payload.clone()).is_none(),
                "a non-object payload must not become ProductGroupData::Other: {payload}"
            );
        }
    }

    /// An object for an untyped product group is still accepted, tag or not.
    #[test]
    fn an_object_for_an_unknown_product_group_is_still_accepted() {
        let tagged =
            ProductGroupData::other(json!({ "productGroup": "quantum-widget", "spinPct": 3 }))
                .expect("an unknown tagged product_group is representable");
        assert_eq!(
            tagged.product_group(),
            ProductGroup::Other("quantum-widget".into())
        );

        let untagged = ProductGroupData::other(json!({ "spinPct": 3 }))
            .expect("an untagged object defaults to other");
        assert_eq!(
            untagged.product_group(),
            ProductGroup::Other("other".into())
        );
    }

    /// A typed product group's tag is still refused — the pre-existing rule.
    #[test]
    fn a_typed_product_groups_tag_is_still_refused() {
        assert!(ProductGroupData::other(json!({ "productGroup": "battery" })).is_none());
    }

    /// Everything this constructor accepts round-trips.
    ///
    /// The property the object guard buys: previously a caller could build a
    /// value that serialised to something `Deserialize` then rejected.
    #[test]
    fn everything_accepted_round_trips() {
        for payload in [
            json!({ "productGroup": "quantum-widget", "spinPct": 3 }),
            json!({ "spinPct": 3 }),
            json!({}),
        ] {
            let Some(built) = ProductGroupData::other(payload.clone()) else {
                continue;
            };
            let wire = serde_json::to_value(&built).expect("serialises");
            assert!(
                wire.get("productGroup")
                    .and_then(serde_json::Value::as_str)
                    .is_some(),
                "the serialised form must carry a product_group tag: {wire}"
            );
            let back: ProductGroupData = serde_json::from_value(wire).expect("round-trips");
            assert_eq!(back, built);
        }
    }
}
