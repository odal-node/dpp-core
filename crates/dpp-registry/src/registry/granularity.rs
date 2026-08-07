//! The level at which a digital product passport is registered, and the
//! higher-level identifiers a registration at that level must carry.
//!
//! **Where this comes from.** Commission Implementing Regulation (EU) 2026/1778
//! lays down the implementation arrangements for the ESPR Art. 13 registry.
//! Its **Art. 8(1)** requires a passport to be registered "at the level
//! specified in the applicable delegated acts (model, batch or item level)",
//! and **Art. 8(3)** resolves a product caught by several Union rules to "the
//! most granular level required".
//!
//! Two linking obligations follow, and they are the reason this is a type
//! rather than a string:
//!
//! - **Art. 8(4)** — where the passport is created at item level, "both batch
//!   and model identifiers shall be linked to that digital product passport
//!   **where batch and model design exist for the product**".
//! - **Art. 8(5)** — where it is created at batch level, "the model identifier
//!   shall be linked … **where model design exists for the product**".
//!
//! Both obligations are conditional on the design existing. Recital (14) is
//! explicit that "for products that are unique by nature, including handmade
//! goods, no batch and model identifiers are required". So absence is a lawful
//! state and is modelled as `None`; what is *not* lawful is claiming an
//! identifier and leaving it blank, or carrying an identifier finer than the
//! level being registered.

use serde::{Deserialize, Serialize};

use super::error::RegistryValidationError;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn granularity_round_trips_through_its_wire_form() {
        for level in [Granularity::Model, Granularity::Batch, Granularity::Item] {
            let json = serde_json::to_string(&level).unwrap();
            assert_eq!(json, format!("\"{}\"", level.wire_str()));
            assert_eq!(serde_json::from_str::<Granularity>(&json).unwrap(), level);
        }
    }

    /// Art. 8(4): an item-level passport links both batch and model.
    #[test]
    fn item_level_may_link_both_batch_and_model() {
        let level = RegistrationLevel::new(Granularity::Item)
            .with_model("MODEL-1")
            .with_batch("BATCH-42");
        assert!(level.validate().is_ok());
    }

    /// Recital (14): products unique by nature carry neither identifier, so
    /// absence must validate.
    #[test]
    fn a_product_unique_by_nature_links_neither_identifier() {
        let level = RegistrationLevel::new(Granularity::Item);
        assert!(
            level.validate().is_ok(),
            "handmade goods have no batch or model design; absence is lawful"
        );
    }

    /// A blank identifier claims a design exists and then fails to name it.
    #[test]
    fn a_blank_linked_identifier_is_refused() {
        for level in [
            RegistrationLevel::new(Granularity::Item).with_model("   "),
            RegistrationLevel::new(Granularity::Item).with_batch(""),
        ] {
            assert!(
                matches!(
                    level.validate(),
                    Err(RegistryValidationError::MissingRequiredField(_))
                ),
                "a claimed-but-blank identifier must be refused: {level:?}"
            );
        }
    }

    /// Art. 8(1)/8(7)(c): the level a registration declares must not be
    /// contradicted by a finer identifier travelling with it.
    #[test]
    fn a_model_level_registration_cannot_carry_a_batch_identifier() {
        let level = RegistrationLevel::new(Granularity::Model).with_batch("BATCH-42");
        assert!(
            matches!(
                level.validate(),
                Err(RegistryValidationError::GranularityMismatch { .. })
            ),
            "a model-level registration covers every batch; naming one contradicts the level"
        );
    }

    #[test]
    fn a_batch_level_registration_may_carry_its_own_batch_identifier() {
        let level = RegistrationLevel::new(Granularity::Batch)
            .with_batch("BATCH-42")
            .with_model("MODEL-1");
        assert!(level.validate().is_ok());
    }
}
