//! [`TransferRecord`] — a single transfer-of-responsibility event.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::TransferError;
use super::operator::ResponsibleOperator;
use super::status::TransferStatus;
use crate::passport::PassportId;

/// The reason for a transfer of DPP responsibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum TransferReason {
    /// Product sold to a new economic operator for market placement.
    Sale,
    /// Product returned to the supply chain (e.g. customer return).
    Return,
    /// Product sent for remanufacturing.
    Remanufacturing,
    /// Product adapted for a different purpose.
    Repurposing,
    /// Product prepared for resale as second-hand.
    PreparationForReuse,
    /// Product imported into the EU by a new importer.
    Import,
    /// Original operator became insolvent; responsibilities assumed by successor.
    InsolvencySuccession,
}

impl TransferReason {
    /// Every reason this build models, for exhaustive iteration.
    ///
    /// `TransferReason` is `#[non_exhaustive]`, so a consumer outside this crate
    /// cannot enumerate it, and one publishing an API description has to. See
    /// [`crate::seal::SealFormat::ALL`] for the same contract: a reason
    /// added later is deliberately not covered until it is added here.
    pub const ALL: &'static [Self] = &[
        Self::Sale,
        Self::Return,
        Self::Remanufacturing,
        Self::Repurposing,
        Self::PreparationForReuse,
        Self::Import,
        Self::InsolvencySuccession,
    ];

    /// The stable wire form, for payloads that carry the reason as a string.
    ///
    /// Spelled out rather than derived from `Serialize` so that renaming a
    /// variant cannot silently change what a registry receives.
    pub fn wire_str(&self) -> &'static str {
        match self {
            Self::Sale => "sale",
            Self::Return => "return",
            Self::Remanufacturing => "remanufacturing",
            Self::Repurposing => "repurposing",
            Self::PreparationForReuse => "preparationForReuse",
            Self::Import => "import",
            Self::InsolvencySuccession => "insolvencySuccession",
        }
    }
}

/// A single transfer-of-responsibility event in the DPP lifecycle.
///
/// Recorded in two steps — the outgoing operator authorises the handover, and
/// the hosting node then attests that the incoming operator accepted it. The
/// two are **not** symmetric: see [`TransferRecord::from_signature`] and
/// [`TransferRecord::node_acceptance_attestation`], only the first of which is
/// a party's own signature.
///
/// The chain this builds is an auditable local record of what the node was
/// told. It is not proof of who holds the obligations: under Implementing
/// Regulation (EU) 2026/1778 Art. 6a that is the EU registry's record, between
/// actors whose identity is verified by eIDAS means under its Arts. 4-5.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRecord {
    /// Unique identifier for this transfer event.
    pub transfer_id: Uuid,
    /// The passport being transferred.
    pub passport_id: PassportId,
    /// The outgoing (previous) responsible operator.
    pub from_operator: ResponsibleOperator,
    /// The incoming (new) responsible operator.
    pub to_operator: ResponsibleOperator,
    /// The reason for this transfer.
    pub reason: TransferReason,
    /// Compact JWS signature from the outgoing operator, signing over the
    /// transfer payload to authorise the handover.
    ///
    /// Produced by the node that hosts the outgoing operator, using that
    /// operator's key — so in the managed single-node model this genuinely is
    /// the outgoing party's signature. Its counterpart below is not.
    pub from_signature: Option<String>,
    /// This **node's** attestation that the incoming operator accepted the
    /// handover. **Not a signature by the incoming operator.**
    ///
    /// The incoming operator is a party with no key on this node, so nothing
    /// here can be signed by them. What is recorded is a JWS over the same
    /// [`Self::signing_payload`], produced by the same key that produced
    /// [`Self::from_signature`], at the moment the acceptance step ran. Because
    /// the payload is RFC 8785 canonical, the JWS header carries no nonce or
    /// timestamp, and Ed25519 is deterministic, the two values are **byte-
    /// identical** in the managed model. The second one therefore carries
    /// exactly one bit — that the acceptance step ran — and `completed_at`
    /// carries the same bit with a timestamp attached.
    ///
    /// It is named for what it is because the previous name, `to_signature`,
    /// was not. That name invited every consumer — including the registry
    /// notification built from this record — to read the value as the incoming
    /// operator's own authorisation, which is a claim no node can make about a
    /// counterparty whose key it does not hold.
    ///
    /// **The authoritative record of who holds the obligations is the EU
    /// registry**, under Implementing Regulation (EU) 2026/1778 Art. 6a, between
    /// actors whose identity is verified by eIDAS means under its Arts. 4–5.
    /// A chain of `did:web` identifiers is not evidence of that standing. See
    /// `docs/regulatory/COMPLIANCE.md` § Transfer-of-Responsibility Article Pin.
    ///
    /// `alias` keeps chains stored under the old key readable: without it every
    /// already-completed transfer would deserialize with this field `None` and
    /// [`Self::is_complete`] would start answering `false` for handovers that
    /// did complete.
    #[serde(alias = "toSignature")]
    pub node_acceptance_attestation: Option<String>,
    /// Timestamp when the transfer was initiated.
    pub initiated_at: DateTime<Utc>,
    /// Timestamp when the transfer was completed — the outgoing operator had
    /// authorised it and the acceptance step had run.
    /// `None` if the transfer is still pending acceptance.
    pub completed_at: Option<DateTime<Utc>>,
    /// Timestamp when the incoming operator explicitly rejected the transfer.
    /// Set by [`TransferRecord::reject`]; makes the record terminal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_at: Option<DateTime<Utc>>,
    /// Timestamp when the outgoing operator cancelled the transfer.
    /// Set by [`TransferRecord::cancel`]; makes the record terminal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<DateTime<Utc>>,
    /// Free-text notes (e.g. conditions, regulatory references).
    pub notes: Option<String>,
}

impl TransferRecord {
    /// The canonical content signed over at both steps: the immutable core
    /// of the transfer, excluding the signatures themselves and the lifecycle
    /// timestamps set *after* signing (`completed_at`/`rejected_at`/`cancelled_at`).
    ///
    /// [`Self::from_signature`] and [`Self::node_acceptance_attestation`] are
    /// both JWS over the JCS canonicalisation of this value, so both bind the
    /// same immutable handover terms and tampering any bound field invalidates
    /// both.
    ///
    /// Note what that implies: the same payload signed by the same key yields
    /// the same JWS, so in the managed single-node model the two values are
    /// byte-identical. That is a property of this design rather than a defect —
    /// the second records that the acceptance step ran, and nothing more.
    #[must_use]
    pub fn signing_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "transferId": self.transfer_id,
            "passportId": self.passport_id,
            "fromOperator": self.from_operator,
            "toOperator": self.to_operator,
            "reason": self.reason,
            "initiatedAt": self.initiated_at,
        })
    }

    /// Determine the current status of this transfer.
    ///
    /// Terminal states (`Rejected`, `Cancelled`) take priority over signatures,
    /// so a cancelled transfer that already had the from_signature still reports
    /// `Cancelled` rather than `Initiated`.
    pub fn status(&self) -> TransferStatus {
        if self.rejected_at.is_some() {
            return TransferStatus::Rejected;
        }
        if self.cancelled_at.is_some() {
            return TransferStatus::Cancelled;
        }
        match (
            &self.from_signature,
            &self.node_acceptance_attestation,
            &self.completed_at,
        ) {
            (Some(_), Some(_), Some(_)) => TransferStatus::Completed,
            (Some(_), Some(_), None) => TransferStatus::Accepted,
            _ => TransferStatus::Initiated,
        }
    }

    /// Returns `true` if the handover was authorised, accepted and finalised.
    pub fn is_complete(&self) -> bool {
        self.from_signature.is_some()
            && self.node_acceptance_attestation.is_some()
            && self.completed_at.is_some()
    }

    /// The incoming operator explicitly rejects the transfer.
    ///
    /// Only valid from `Initiated` state. After rejection the record is terminal;
    /// a new transfer may be initiated on the chain.
    pub fn reject(&mut self) -> Result<(), TransferError> {
        let s = self.status();
        if s != TransferStatus::Initiated {
            return Err(TransferError::InvalidState {
                current: s,
                action: "reject".into(),
            });
        }
        self.rejected_at = Some(Utc::now());
        Ok(())
    }

    /// The outgoing operator cancels the transfer before it completes.
    ///
    /// Valid from `Initiated` or `Accepted` state. After cancellation the
    /// record is terminal; a new transfer may be initiated on the chain.
    pub fn cancel(&mut self) -> Result<(), TransferError> {
        match self.status() {
            TransferStatus::Initiated | TransferStatus::Accepted => {
                self.cancelled_at = Some(Utc::now());
                Ok(())
            }
            s => Err(TransferError::InvalidState {
                current: s,
                action: "cancel".into(),
            }),
        }
    }

    /// Mark the transfer as completed once it is authorised and accepted.
    ///
    /// Only valid from `Accepted` state ([`Self::from_signature`] and
    /// [`Self::node_acceptance_attestation`] both present, no `completed_at`
    /// yet). This is the final step before the incoming operator becomes the
    /// current responsible operator **in this node's chain** — which is a local
    /// record, not the registry's determination of who holds the obligations.
    pub fn complete(&mut self) -> Result<(), TransferError> {
        let s = self.status();
        if s != TransferStatus::Accepted {
            return Err(TransferError::InvalidState {
                current: s,
                action: "complete".into(),
            });
        }
        self.completed_at = Some(Utc::now());
        Ok(())
    }
}
