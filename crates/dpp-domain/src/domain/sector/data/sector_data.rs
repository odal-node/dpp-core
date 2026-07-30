//! The [`SectorData`] discriminated union and its per-audience redaction.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::aluminium::AluminiumData;
use super::battery::BatteryData;
use super::construction::ConstructionData;
use super::detergent::DetergentData;
use super::electronics::ElectronicsData;
use super::furniture::FurnitureData;
use super::steel::SteelData;
use super::textile::TextileData;
use super::toy::ToyData;
use super::tyre::TyreData;
use super::unsold_goods::UnsoldGoodsReport;
use crate::domain::sector::Sector;

/// Typed, sector-specific DPP data — replaces the opaque `compliance_data: Value`.
///
/// Serialises as an internally-tagged object where `"sector"` is the
/// discriminant field, e.g.:
/// ```json
/// { "sector": "battery", "gtin": "09506000134352", "nominalVoltageV": 3.2, ... }
/// ```
/// ```json
/// { "sector": "textile", "fibreComposition": [...], "countryOfManufacturing": "BD" }
/// ```
/// An unknown `sector` tag deserialises to [`SectorData::Other`], which keeps
/// both the tag and the payload verbatim — see the hand-written `Deserialize`
/// below for why the derive cannot do this.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SectorData {
    Battery(BatteryData),
    Textile(TextileData),
    UnsoldGoods(UnsoldGoodsReport),
    Steel(SteelData),
    Electronics(ElectronicsData),
    Construction(ConstructionData),
    Tyre(TyreData),
    Toy(ToyData),
    Aluminium(AluminiumData),
    Furniture(FurnitureData),
    Detergent(DetergentData),
    /// A product group this build has no typed variant for.
    ///
    /// `sector` is the wire tag verbatim and `data` is the whole object. A
    /// passport for a sector added to the catalog after this crate was
    /// released survives a round trip unchanged — which is the property that
    /// makes adding a product group a data change rather than a release.
    Other {
        /// The wire tag exactly as received.
        sector: String,
        /// The full object, including its `sector` key.
        data: serde_json::Value,
    },
}

// ─── Wire format ─────────────────────────────────────────────────────────────
//
// Hand-written because the sector tag is open. `#[serde(tag = "sector")]`
// enumerates its variants at compile time and rejects anything else, which
// would make the wire the one closed part of an otherwise data-driven sector
// model: the catalog, the schema registry and the plugin manifests are all
// keyed by string, so a product group added to the catalog would still fail to
// deserialize until this crate was released.
//
// The shape is unchanged — an internally-tagged object with `sector` as the
// discriminant. Only the unknown-tag behaviour differs: fall through to
// `Other`, keeping tag and payload, instead of failing.

impl Serialize for SectorData {
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
            Self::Detergent(d) => serde_json::to_value(d),
            Self::Other { data, .. } => Ok(data.clone()),
        }
        .map_err(serde::ser::Error::custom)?;

        let tag = self.sector();
        if let serde_json::Value::Object(ref mut map) = value {
            map.insert(
                "sector".to_owned(),
                serde_json::Value::String(tag.wire_str().to_owned()),
            );
        }
        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SectorData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;

        let value = serde_json::Value::deserialize(deserializer)?;
        let tag = value
            .get("sector")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| D::Error::missing_field("sector"))?
            .to_owned();

        // A typed variant ignores the `sector` key it does not declare, so the
        // whole object can be handed to it unchanged.
        macro_rules! typed {
            ($variant:ident) => {
                serde_json::from_value(value.clone())
                    .map(Self::$variant)
                    .map_err(D::Error::custom)
            };
        }

        match Sector::from_wire_tag(&tag) {
            Sector::Battery => typed!(Battery),
            Sector::Textile => typed!(Textile),
            Sector::UnsoldGoods => typed!(UnsoldGoods),
            Sector::Steel => typed!(Steel),
            Sector::Electronics => typed!(Electronics),
            Sector::Construction => typed!(Construction),
            Sector::Tyre => typed!(Tyre),
            Sector::Toy => typed!(Toy),
            Sector::Aluminium => typed!(Aluminium),
            Sector::Furniture => typed!(Furniture),
            Sector::Detergent => typed!(Detergent),
            Sector::Other(sector) => Ok(Self::Other {
                sector,
                data: value,
            }),
        }
    }
}

impl SectorData {
    /// Build an untyped payload, reading the tag from the object's own
    /// `sector` field.
    ///
    /// Falls back to `"other"` only when the object does not carry one — a
    /// payload with no tag has no identity to preserve.
    #[must_use]
    pub fn other(data: serde_json::Value) -> Self {
        let sector = data
            .get("sector")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("other")
            .to_owned();
        Self::Other { sector, data }
    }

    /// Returns the `Sector` discriminant for this data.
    pub fn sector(&self) -> Sector {
        match self {
            SectorData::Battery(_) => Sector::Battery,
            SectorData::Textile(_) => Sector::Textile,
            SectorData::UnsoldGoods(_) => Sector::UnsoldGoods,
            SectorData::Steel(_) => Sector::Steel,
            SectorData::Electronics(_) => Sector::Electronics,
            SectorData::Construction(_) => Sector::Construction,
            SectorData::Tyre(_) => Sector::Tyre,
            SectorData::Toy(_) => Sector::Toy,
            SectorData::Aluminium(_) => Sector::Aluminium,
            SectorData::Furniture(_) => Sector::Furniture,
            SectorData::Detergent(_) => Sector::Detergent,
            SectorData::Other { sector, .. } => Sector::Other(sector.clone()),
        }
    }

    /// The GTIN carried by this sector's typed data, if any.
    ///
    /// `UnsoldGoods` and `Other` carry no GTIN field — a discard-event report
    /// and an untyped catch-all respectively, neither of which identifies a
    /// trade item the way every other sector does.
    pub fn gtin(&self) -> Option<&str> {
        match self {
            SectorData::Battery(d) => Some(d.gtin.as_str()),
            SectorData::Textile(d) => Some(d.gtin.as_str()),
            SectorData::Steel(d) => Some(d.gtin.as_str()),
            SectorData::Electronics(d) => Some(d.gtin.as_str()),
            SectorData::Construction(d) => Some(d.gtin.as_str()),
            SectorData::Tyre(d) => Some(d.gtin.as_str()),
            SectorData::Toy(d) => Some(d.gtin.as_str()),
            SectorData::Aluminium(d) => Some(d.gtin.as_str()),
            SectorData::Furniture(d) => Some(d.gtin.as_str()),
            SectorData::Detergent(d) => Some(d.gtin.as_str()),
            SectorData::UnsoldGoods(_) | SectorData::Other { .. } => None,
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
pub fn redact_sector_data(
    data: &SectorData,
    audience: crate::domain::identity::Audience,
    descriptor: &crate::catalog::SectorDescriptor,
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
