//! [`Regime`] — which EU legal instrument a sector's DPP obligation derives from.

use serde::{Deserialize, Serialize};

/// The legal instrument family a sector belongs to.
///
/// Distinct from [`crate::catalog::RegulatoryStatus`], which says *whether* a
/// sector's obligations bind. This says *which law* they come from. The two are
/// independent: a sector can be `Provisional` under ESPR and another
/// `Provisional` under the PPWR, and the determination gate must treat them
/// identically while the catalog still records where each came from.
///
/// ESPR is not the only source of a passport obligation. Of the sectors this
/// crate ships, several derive from standalone instruments — batteries, toys,
/// detergents and construction products each have their own regulation, and
/// electronics rests on the ecodesign and energy-labelling pair.
/// Serialised as the bare manifest string, with any unmodelled value absorbed by
/// [`Regime::Other`]. The derived representation could not do this: an
/// externally-tagged `Other(String)` renders as `{"other": "..."}` rather than a
/// string, and an unrecognised value failed the whole deserialise instead of
/// reaching `Other` — so a manifest naming a new instrument broke catalog
/// loading outright, and `Other` was unreachable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
#[non_exhaustive]
pub enum Regime {
    /// Ecodesign for Sustainable Products Regulation (EU) 2024/1781, including
    /// the Art. 24/25 unsold-goods obligations and working-plan product groups.
    Espr,
    /// Batteries Regulation (EU) 2023/1542.
    BatteryRegulation,
    /// Toy Safety Regulation (EU) 2025/2509.
    ToySafety,
    /// Detergents Regulation (EU) 2026/405.
    Detergents,
    /// Construction Products Regulation (EU) 2024/3110.
    Cpr,
    /// Packaging and Packaging Waste Regulation (EU) 2025/40.
    Ppwr,
    /// End-of-Life Vehicles Regulation — the Circularity Vehicle Passport.
    Elv,
    /// Product-specific ecodesign and energy-labelling implementing acts, e.g.
    /// Regulations (EU) 2023/1670 and 2023/1669 for smartphones and tablets.
    EcodesignEnergyLabelling,
    /// A named instrument this enum does not yet model.
    Other(String),
    /// Tracked, but no DPP instrument exists. Pairs with
    /// [`crate::catalog::RegulatoryStatus::Watch`].
    None,
}

impl Regime {
    /// Manifest spelling of a modelled variant. `None` for [`Regime::Other`],
    /// which carries its own string.
    #[must_use]
    pub fn wire_str(&self) -> Option<&'static str> {
        Some(match self {
            Self::Espr => "espr",
            Self::BatteryRegulation => "battery-regulation",
            Self::ToySafety => "toy-safety",
            Self::Detergents => "detergents",
            Self::Cpr => "cpr",
            Self::Ppwr => "ppwr",
            Self::Elv => "elv",
            Self::EcodesignEnergyLabelling => "ecodesign-energy-labelling",
            Self::None => "none",
            Self::Other(_) => return Option::None,
        })
    }
}

impl From<String> for Regime {
    /// Unrecognised values become [`Regime::Other`] rather than failing: a
    /// manifest naming an instrument this enum does not yet model must still
    /// load, and the value must survive verbatim.
    fn from(s: String) -> Self {
        match s.as_str() {
            "espr" => Self::Espr,
            "battery-regulation" => Self::BatteryRegulation,
            "toy-safety" => Self::ToySafety,
            "detergents" => Self::Detergents,
            "cpr" => Self::Cpr,
            "ppwr" => Self::Ppwr,
            "elv" => Self::Elv,
            "ecodesign-energy-labelling" => Self::EcodesignEnergyLabelling,
            "none" => Self::None,
            _ => Self::Other(s),
        }
    }
}

impl From<Regime> for String {
    fn from(regime: Regime) -> Self {
        match regime {
            Regime::Other(s) => s,
            modelled => modelled
                .wire_str()
                .expect("every non-Other variant has a wire string")
                .to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_modelled_variant_round_trips_as_a_bare_string() {
        for regime in [
            Regime::Espr,
            Regime::BatteryRegulation,
            Regime::ToySafety,
            Regime::Detergents,
            Regime::Cpr,
            Regime::Ppwr,
            Regime::Elv,
            Regime::EcodesignEnergyLabelling,
            Regime::None,
        ] {
            let json = serde_json::to_string(&regime).expect("serialise");
            assert!(
                json.starts_with('"'),
                "{regime:?} must render as a string, got {json}"
            );
            assert_eq!(
                serde_json::from_str::<Regime>(&json).expect("deserialise"),
                regime
            );
        }
    }

    #[test]
    fn an_unmodelled_instrument_is_absorbed_not_rejected() {
        // A manifest naming a regime this build does not know must still load.
        let parsed: Regime =
            serde_json::from_str("\"circular-economy-act\"").expect("must not fail");
        assert_eq!(parsed, Regime::Other("circular-economy-act".to_owned()));
        // …and must survive the round trip verbatim, not become {"other": …}.
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            "\"circular-economy-act\""
        );
    }
}
