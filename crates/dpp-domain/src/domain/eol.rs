//! End-of-life declarations for a Digital Product Passport.
//!
//! A DPP is never *deleted* at end of life — the passport outlives the product
//! (EN 18221 retention posture). Instead the passport transitions to
//! [`super::status::PassportStatus::Deactivated`] and carries a typed
//! [`EolEvent`] recording *why* and, for circularity (ESPR / Battery Annex XIII),
//! what material was recovered. Destruction specifically must cite a recognised
//! derogation from the unsold-goods destruction ban (ESPR Art. 25 delegated act).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::passport::PassportId;

/// A recognised derogation from the ESPR Art. 25 destruction ban. The exact
/// category list is fixed by the Feb-2026 delegated act; the category string is
/// validated against that list at the engine boundary, not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerogationRef {
    /// The derogation category as named by the delegated act.
    pub category: String,
    /// The act/article this derogation is grounded in (e.g. an OJ/CELEX ref).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act_citation: Option<String>,
}

/// Why a passport reached end of life. Destruction requires a [`DerogationRef`]
/// so a record can never claim destruction without citing a lawful basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[non_exhaustive]
pub enum DeactivationReason {
    /// Sent for material recovery (preferred circular outcome).
    Recycled,
    /// Destroyed — only lawful with a recognised derogation from the ban.
    Destroyed {
        /// The derogation category authorising destruction.
        derogation: DerogationRef,
    },
    /// Exported out of the EU market.
    Exported,
    /// Product lost (theft, disaster) — recorded, not silently dropped.
    Lost,
}

impl DeactivationReason {
    /// Every `kind` discriminator this build models, for exhaustive iteration.
    ///
    /// Discriminator strings rather than `&[Self]` because `Destroyed` carries a
    /// [`DerogationRef`] and so has no value-free form — the tag is the part a
    /// consumer needs to enumerate.
    ///
    /// `DeactivationReason` is `#[non_exhaustive]`, so a consumer outside this
    /// crate cannot enumerate it, and one publishing an API description has to.
    /// See [`crate::domain::seal::SealFormat::ALL`] for the same contract: a
    /// reason added later is deliberately not covered until it is added here.
    pub const KINDS: &'static [&'static str] = &["recycled", "destroyed", "exported", "lost"];
}

/// The end-of-life event attached to a passport when it is deactivated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EolEvent {
    /// The passport being declared end-of-life.
    pub passport_id: PassportId,
    /// The typed reason for deactivation.
    pub reason: DeactivationReason,
    /// DID of the operator declaring EOL (provenance).
    pub declared_by: String,
    /// When EOL was declared.
    pub declared_at: DateTime<Utc>,
    /// Optional recovered-material summary for circularity reporting
    /// (recovered-content shares etc.; Battery Annex XIII). Free-form here; a
    /// sector schema constrains it where the act demands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_recovery: Option<serde_json::Value>,
    /// Free-text notes (conditions, references).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl EolEvent {
    /// Construct an EOL event stamped `declared_at = now`.
    #[must_use]
    pub fn new(
        passport_id: PassportId,
        reason: DeactivationReason,
        declared_by: impl Into<String>,
    ) -> Self {
        Self {
            passport_id,
            reason,
            declared_by: declared_by.into(),
            declared_at: Utc::now(),
            material_recovery: None,
            notes: None,
        }
    }

    /// True when this EOL is a destruction — which must carry a derogation.
    #[must_use]
    pub fn requires_derogation(&self) -> bool {
        matches!(self.reason, DeactivationReason::Destroyed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recycled_roundtrips_and_needs_no_derogation() {
        let e = EolEvent::new(PassportId::new(), DeactivationReason::Recycled, "did:web:r");
        assert!(!e.requires_derogation());
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["reason"]["kind"], "recycled");
        let back: EolEvent = serde_json::from_value(v).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn destroyed_carries_a_derogation() {
        let e = EolEvent::new(
            PassportId::new(),
            DeactivationReason::Destroyed {
                derogation: DerogationRef {
                    category: "health-and-safety".into(),
                    act_citation: Some("Delegated Reg. (EU) 2026/xxx".into()),
                },
            },
            "did:web:r",
        );
        assert!(e.requires_derogation());
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["reason"]["kind"], "destroyed");
        assert_eq!(v["reason"]["derogation"]["category"], "health-and-safety");
    }
}

#[cfg(test)]
mod deactivation_reason_kinds_tests {
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
}
