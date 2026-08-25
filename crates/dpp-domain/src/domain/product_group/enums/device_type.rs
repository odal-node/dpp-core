//! [`DeviceType`] — the Regulation (EU) 2023/1670 Art. 1(1) device categories.

use serde::{Deserialize, Serialize};

/// Electronics device type per EU Regulation (EU) 2023/1670 Art. 1(1).
///
/// **Deliberately no `#[serde(other)]` catch-all**, same reasoning as
/// [`BatteryType`](crate::domain::product_group::BatteryType). Art. 1(1) enumerates exactly four device types —
/// smartphones, other mobile phones, cordless phones, and slate tablets —
/// and no others are within this regulation's scope. `laptop`, `monitor`,
/// `tv`, `server`, `charger`, `earphone`, `router` and `pcb` previously
/// appeared here or in the schema enum with no regulatory basis; see
/// `product-groups/electronics.json`'s `notes` for where (if anywhere) each of
/// those actually belongs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DeviceType {
    Smartphone,
    OtherMobilePhone,
    CordlessPhone,
    /// Art. 1(1) calls this a "slate tablet"; kept as `tablet` on the wire —
    /// the value this product group already used before the narrowing, unchanged.
    #[serde(rename = "tablet")]
    Tablet,
}
