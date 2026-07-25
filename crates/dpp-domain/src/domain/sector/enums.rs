//! Cross-sector typed enumerations (3.2d).
//!
//! Shared enums referenced by more than one sector's data struct (chemistry,
//! production route, energy/carbon classes, LCA boundaries).

use serde::{Deserialize, Serialize};

/// Battery electrochemical chemistry with `#[serde(other)]` fallback for future types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BatteryChemistry {
    #[serde(rename = "LFP")]
    Lfp,
    #[serde(rename = "NMC")]
    Nmc,
    #[serde(rename = "NCA")]
    Nca,
    #[serde(rename = "LCO")]
    Lco,
    #[serde(rename = "NiMH")]
    NiMh,
    #[serde(rename = "NiCd")]
    NiCd,
    #[serde(rename = "lead-acid")]
    LeadAcid,
    #[serde(rename = "solid-state")]
    SolidState,
    /// Absorbs unknown chemistry codes on deserialization (forward compatibility).
    #[serde(other)]
    Other,
}

impl BatteryChemistry {
    /// The serde wire tag for this chemistry code, e.g. `"LFP"`, `"lead-acid"`.
    /// Equivalent to `serde_json::to_value(self)` but without the allocation
    /// and `Value` round trip.
    pub const fn wire_str(&self) -> &'static str {
        match self {
            Self::Lfp => "LFP",
            Self::Nmc => "NMC",
            Self::Nca => "NCA",
            Self::Lco => "LCO",
            Self::NiMh => "NiMH",
            Self::NiCd => "NiCd",
            Self::LeadAcid => "lead-acid",
            Self::SolidState => "solid-state",
            Self::Other => "Other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_chemistry_wire_str_matches_serde_serialization() {
        for chem in [
            BatteryChemistry::Lfp,
            BatteryChemistry::Nmc,
            BatteryChemistry::Nca,
            BatteryChemistry::Lco,
            BatteryChemistry::NiMh,
            BatteryChemistry::NiCd,
            BatteryChemistry::LeadAcid,
            BatteryChemistry::SolidState,
            BatteryChemistry::Other,
        ] {
            let serialized = serde_json::to_value(&chem).unwrap();
            assert_eq!(
                serialized.as_str().unwrap(),
                chem.wire_str(),
                "wire_str() disagrees with serde for {chem:?}"
            );
        }
    }
}

/// Battery type category per EU Battery Regulation 2023/1542 Art. 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BatteryType {
    Portable,
    Industrial,
    Ev,
    Lmt,
    /// Starting, lighting, and ignition batteries.
    #[serde(rename = "starting-lighting-ignition")]
    Sli,
    #[serde(other)]
    Other,
}

/// Error from constructing a [`CarbonFootprintClass`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CarbonFootprintClassError {
    #[error("carbon footprint class label must not be empty or blank")]
    Empty,
    #[error(
        "carbon footprint class label '{label}' is {len} characters, \
         exceeding the maximum of {max}"
    )]
    TooLong {
        label: String,
        len: usize,
        max: usize,
    },
    #[error("carbon footprint class label '{0}' contains a control character")]
    ControlCharacter(String),
}

/// A carbon footprint performance class label, preserved verbatim as declared.
///
/// **Deliberately not an enumeration.** Art. 7(2) of Regulation (EU) 2023/1542
/// defines no class labels — it defers them to a delegated act that has not been
/// adopted, and in the same paragraph requires the Commission to "review the
/// number of performance classes and the thresholds between them, every three
/// years". A fixed variant set is therefore wrong on a three-year cycle.
///
/// An `#[serde(other)]` catch-all is worse than wrong: an earlier version of
/// this type mapped every unrecognised label to `Other`, discarding the declared
/// string. Since a published passport carries a qualified electronic seal, a
/// lossy round-trip is a correctness defect — "F" and "A+" under a future
/// seven-class scale would both have been stored, and re-served, as `Other`.
///
/// A label means nothing on its own: it is only interpretable against the
/// ruleset whose boundaries produced it. Always carry it alongside
/// [`BatteryData::carbon_footprint_class_ruleset_id`](crate::BatteryData) and
/// its version.
///
/// The length bound matches battery schema v2.1.0 (`maxLength: 8`), so a value
/// that validates here also validates against the schema.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CarbonFootprintClass(String);

impl CarbonFootprintClass {
    /// Maximum label length in characters, matching the schema's `maxLength`.
    pub const MAX_LEN: usize = 8;

    /// Construct a class label, rejecting anything the schema would reject.
    ///
    /// The label is stored exactly as given — no case folding, no trimming of
    /// interior content — because the declared value is what an auditor
    /// re-checks against the delegated act.
    pub fn new(label: impl Into<String>) -> Result<Self, CarbonFootprintClassError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(CarbonFootprintClassError::Empty);
        }
        if label.chars().any(char::is_control) {
            return Err(CarbonFootprintClassError::ControlCharacter(label));
        }
        let len = label.chars().count();
        if len > Self::MAX_LEN {
            return Err(CarbonFootprintClassError::TooLong {
                label,
                len,
                max: Self::MAX_LEN,
            });
        }
        Ok(Self(label))
    }

    /// The label as declared.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CarbonFootprintClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for CarbonFootprintClass {
    type Error = CarbonFootprintClassError;

    fn try_from(label: String) -> Result<Self, Self::Error> {
        Self::new(label)
    }
}

impl From<CarbonFootprintClass> for String {
    fn from(class: CarbonFootprintClass) -> Self {
        class.0
    }
}

/// EU energy label class per EU Energy Labelling Regulation 2017/1369 (A–G scale).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EnergyEfficiencyClass {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    #[serde(other)]
    Other,
}

/// Steel and aluminium production route — determines carbon intensity basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ProductionRoute {
    /// Integrated blast furnace / basic oxygen furnace (steel).
    BlastFurnace,
    /// Electric arc furnace (steel — typically secondary).
    ElectricArc,
    /// Direct reduced iron route (steel).
    DirectReduction,
    /// Primary Hall-Héroult electrolysis (aluminium).
    Primary,
    /// Secondary recycled route (aluminium).
    SecondaryRecycled,
    Mixed,
    #[serde(other)]
    Other,
}

/// LCA lifecycle stage boundary for a carbon footprint declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum LifecycleStage {
    CradleToGate,
    CradleToGrave,
    CradleToCradle,
    GateToGrave,
    #[serde(other)]
    Other,
}

/// LCA system-boundary standard referenced in a carbon footprint declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SystemBoundary {
    #[serde(rename = "EN-15804")]
    En15804,
    #[serde(rename = "ISO-14044")]
    Iso14044,
    #[serde(rename = "GHG-protocol")]
    GhgProtocol,
    #[serde(other)]
    Other,
}

#[cfg(test)]
mod carbon_footprint_class_tests {
    use super::*;

    #[test]
    fn round_trip_preserves_a_label_this_build_has_never_seen() {
        // The regression this type exists to prevent. The previous enum mapped
        // every unrecognised label to `Other`, so a future scale's "F" and "A+"
        // both round-tripped to the string "Other" — under a qualified seal.
        for label in ["A", "F", "A+", "A+++", "CLASS-1"] {
            let json = serde_json::to_string(&CarbonFootprintClass::new(label).unwrap()).unwrap();
            assert_eq!(json, format!("\"{label}\""));
            let back: CarbonFootprintClass = serde_json::from_str(&json).unwrap();
            assert_eq!(back.as_str(), label, "label must survive the round trip");
        }
    }

    #[test]
    fn deserialization_validates_rather_than_silently_accepting() {
        // A catch-all variant would have swallowed these. `try_from` rejects
        // them at the boundary, where the caller can still report a field path.
        for bad in ["\"\"", "\"   \"", "\"TOOLONGLABEL\""] {
            assert!(
                serde_json::from_str::<CarbonFootprintClass>(bad).is_err(),
                "should reject {bad}"
            );
        }
    }

    #[test]
    fn rejects_empty_blank_overlong_and_control_characters() {
        assert!(matches!(
            CarbonFootprintClass::new(""),
            Err(CarbonFootprintClassError::Empty)
        ));
        assert!(matches!(
            CarbonFootprintClass::new("  \t "),
            Err(CarbonFootprintClassError::Empty)
        ));
        assert!(matches!(
            CarbonFootprintClass::new("A\u{7}"),
            Err(CarbonFootprintClassError::ControlCharacter(_))
        ));
        let err = CarbonFootprintClass::new("ABCDEFGHI").unwrap_err();
        assert!(matches!(
            err,
            CarbonFootprintClassError::TooLong { len: 9, max: 8, .. }
        ));
    }

    #[test]
    fn length_bound_matches_the_schema_and_counts_characters() {
        assert_eq!(CarbonFootprintClass::MAX_LEN, 8);
        // Exactly at the bound is accepted.
        assert!(CarbonFootprintClass::new("ABCDEFGH").is_ok());
        // maxLength in JSON Schema counts code points, not bytes — a
        // byte-length check would wrongly reject this 4-character label.
        assert!(CarbonFootprintClass::new("Ä+++").is_ok());
    }

    #[test]
    fn label_is_stored_verbatim_without_case_folding() {
        let c = CarbonFootprintClass::new("a+").unwrap();
        assert_eq!(c.as_str(), "a+");
        assert_eq!(c.to_string(), "a+");
    }
}
