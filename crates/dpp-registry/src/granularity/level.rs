//! Granularity of a registration and the level it applies at.

use serde::{Deserialize, Serialize};

use crate::error::RegistryValidationError;

/// The registration level of a digital product passport — IR (EU) 2026/1778
/// Art. 8(1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    /// The general design or version of a product: one registration covers
    /// every item sharing those specifications.
    Model,
    /// A specific production run: one registration covers every item made in
    /// that run.
    Batch,
    /// A single physical unit.
    Item,
}

impl Granularity {
    /// The wire form used in registry payloads.
    pub fn wire_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Batch => "batch",
            Self::Item => "item",
        }
    }

    /// How specific this level is: `Model` < `Batch` < `Item`. Used to reject a
    /// registration carrying an identifier finer than the level it declares.
    fn rank(self) -> u8 {
        match self {
            Self::Model => 0,
            Self::Batch => 1,
            Self::Item => 2,
        }
    }
}

impl std::fmt::Display for Granularity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.wire_str())
    }
}

/// The higher-level identifiers Art. 8(4) and 8(5) require a registration to
/// link, alongside the level it is registered at.
///
/// `model_id` and `batch_id` are `Option` because the linking obligation is
/// conditional on a model or batch design existing for the product at all —
/// see the module docs. `None` asserts that no such design exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationLevel {
    /// The level this passport is registered at — Art. 8(1).
    pub granularity: Granularity,
    /// Identifier of the model this product belongs to. `None` only when the
    /// product has no model design (Art. 8(4)/(5), recital (14)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Identifier of the batch this product belongs to. `None` only when the
    /// product has no batch design, and always `None` above batch level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
}

impl RegistrationLevel {
    /// A registration at `granularity` with no higher-level identifiers linked.
    pub fn new(granularity: Granularity) -> Self {
        Self {
            granularity,
            model_id: None,
            batch_id: None,
        }
    }

    /// Link the model this product belongs to.
    #[must_use]
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Link the batch this product belongs to.
    #[must_use]
    pub fn with_batch(mut self, batch_id: impl Into<String>) -> Self {
        self.batch_id = Some(batch_id.into());
        self
    }

    /// Check the Art. 8(4)/(5) linking rules.
    ///
    /// Two classes of error are detectable without knowing whether a model or
    /// batch design exists for this product:
    ///
    /// 1. **A blank claimed identifier.** `Some("")` asserts a design exists
    ///    and then fails to identify it — neither of the two lawful states.
    /// 2. **An identifier finer than the declared level.** A model-level
    ///    registration carrying a batch identifier contradicts its own
    ///    Art. 8(1) level; the registry checks that level on submission under
    ///    Art. 8(7)(c).
    ///
    /// A *missing* `model_id` or `batch_id` is **not** an error here: absence
    /// is the lawful encoding of "no such design exists" (recital (14)), and
    /// this crate cannot know which is the case. The delegated act for the
    /// product group is what settles it.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        for (field, value) in [("modelId", &self.model_id), ("batchId", &self.batch_id)] {
            if value.as_ref().is_some_and(|v| v.trim().is_empty()) {
                return Err(RegistryValidationError::MissingRequiredField(field.into()));
            }
        }

        // A batch identifier is finer than a model-level registration.
        if self.batch_id.is_some() && self.granularity.rank() < Granularity::Batch.rank() {
            return Err(RegistryValidationError::GranularityMismatch {
                granularity: self.granularity.wire_str(),
                identifier: "batchId",
            });
        }

        Ok(())
    }
}
