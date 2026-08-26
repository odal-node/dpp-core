//! Every [`DeactivationReason`] kind is reachable and round-trips.

use super::{DeactivationReason, DerogationRef};

/// `KINDS` must list every discriminator.
///
/// The match is exhaustive with no catch-all, so a new reason stops this
/// compiling; the length assertion then fails until `KINDS` is updated.
#[test]
fn kinds_lists_every_discriminator() {
    let all = [
        DeactivationReason::Recycled,
        DeactivationReason::Destroyed {
            derogation: DerogationRef {
                category: "safety".to_owned(),
                act_citation: None,
            },
        },
        DeactivationReason::Exported,
        DeactivationReason::Lost,
    ];
    for reason in &all {
        match reason {
            DeactivationReason::Recycled
            | DeactivationReason::Destroyed { .. }
            | DeactivationReason::Exported
            | DeactivationReason::Lost => {}
        }
    }
    assert_eq!(
        DeactivationReason::KINDS.len(),
        all.len(),
        "a variant was added to the match above but not to KINDS"
    );

    // The strings must be the ones serde actually emits, not a second
    // transcription of them.
    for reason in &all {
        let tag = serde_json::to_value(reason).expect("serialises")["kind"]
            .as_str()
            .expect("has a kind discriminator")
            .to_owned();
        assert!(
            DeactivationReason::KINDS.contains(&tag.as_str()),
            "serde emits `{tag}`, which KINDS does not list"
        );
    }
}
