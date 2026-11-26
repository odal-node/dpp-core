//! Tests of the `granularity` module.

use super::super::*;
use crate::error::RegistryValidationError;

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
