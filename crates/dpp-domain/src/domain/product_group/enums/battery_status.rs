//! [`BatteryStatus`] — where a battery sits in its life.

use serde::{Deserialize, Serialize};

/// Where a battery is in its life, per Annex XIII point 4(c).
///
/// The annex spells the set out inline — *"the status of the battery, defined
/// as 'original', 'repurposed', 're-used', 'remanufactured' or 'waste'"* — so
/// this is a closed enumeration for the same reason [`BatteryType`] is: the
/// legal text enumerates it and nothing else is a lawful value. No
/// `#[serde(other)]`.
///
/// Point 4 data describes **one physical battery**, not a model, which is why
/// this carries the `individual` disclosure class and is withheld even from
/// authorities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BatteryStatus {
    Original,
    Repurposed,
    #[serde(rename = "re-used")]
    ReUsed,
    Remanufactured,
    Waste,
}
