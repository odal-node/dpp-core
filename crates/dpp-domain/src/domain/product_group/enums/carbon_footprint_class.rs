//! [`CarbonFootprintClass`] — a manufacturer-assigned performance label.
//!
//! ESPR Art. 7(2) defines no labels and requires the class count to be reviewed
//! every three years, so this is a bounded free string, never an enum.

use serde::{Deserialize, Serialize};

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
