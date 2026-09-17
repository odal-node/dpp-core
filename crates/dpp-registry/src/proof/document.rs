//! [`ProofOfRegistration`] — the Art. 9(2) document, as received.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::RegistryValidationError;
use crate::identifiers::OperatorIdentifier;

/// How long the registry keeps a generated proof available, in calendar days.
///
/// ✅ COMPLIANCE-PIN: IR (EU) 2026/1778 **Art. 9(4)** — *"That proof shall remain
/// available for a period of 90 calendar days from the date of its generation."*
///
/// 🚨 **This is the registry's retention window, not the document's validity.**
/// The article governs what the Commission *makes available*; a proof already
/// downloaded does not stop being evidence on day 91, and its qualified seal
/// goes on verifying. What lapses is the ability to fetch it again. A node that
/// treats the horizon as an expiry will discard evidence it is entitled to keep;
/// one that never captures the proof loses the strongest artefact in the system
/// and has ninety days to notice.
pub const AVAILABILITY_DAYS: i64 = 90;

/// Proof that the registration obligation for a passport has been fulfilled.
///
/// ✅ COMPLIANCE-PIN: Commission Implementing Regulation (EU) 2026/1778
/// **Art. 9(2)** — *"The proof of registration shall serve as evidence, including
/// vis-à-vis third parties, that the registration obligation for that digital
/// product passport has been fulfilled."*
///
/// # The field list is a floor, not the document
///
/// 🚨 Art. 9(2) says the proof *"shall contain at least the following data"*.
/// The five points below are a minimum the Commission may exceed, so this type
/// is **not** a complete description of what arrives. Unknown fields deserialise
/// away rather than being refused, deliberately: rejecting a proof for carrying
/// more than the article compels would refuse a lawful document.
///
/// # What is opaque here, and why that is not laziness
///
/// [`qualified_seal`](Self::qualified_seal) and
/// [`commission_time_stamp`](Self::commission_time_stamp) are carried as
/// received and not parsed. Art. 9(3) requires the proof to be *"guaranteed by
/// means of a qualified electronic seal as provided for in Article 38 of
/// Regulation (EU) No 910/2014"* — verifying that is a trust-list question, and
/// this crate holds no trust list and performs no cryptography. A type here that
/// appeared to understand a seal would invite a caller to believe it had been
/// checked.
///
/// [`passport_version_hash`](Self::passport_version_hash) is likewise a string:
/// Art. 9(2)(e) says *"a hash of the version"* and names no algorithm, so
/// parsing one into a typed digest would be reading a choice into the text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ProofOfRegistration {
    /// **(a)** the unique product identifier.
    pub product_identifier: String,
    /// **(b)** the commodity code, *"where relevant"* — Art. 8(9)(b).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commodity_code: Option<String>,
    /// **(c)** the name and identity of the verified economic operator
    /// responsible for the registration, per Art. 8(9)(d).
    ///
    /// Reuses [`OperatorIdentifier`] rather than restating name-and-identity as
    /// loose fields: it is the same party, in the same vocabulary, and a second
    /// shape for it would drift from the one the registration was made under.
    pub operator: OperatorIdentifier,
    /// **(d)** the date and time of registration **for the latest version** of
    /// the passport the proof was generated for.
    ///
    /// Art. 9(2)(d) ties this to the latest version *at generation time*, which
    /// is why a proof for a superseded version keeps its meaning: it is evidence
    /// about that version, not a claim about the passport's current state.
    pub registered_at: DateTime<Utc>,
    /// The Commission's electronic time stamp validating
    /// [`registered_at`](Self::registered_at) — Art. 9(2)(d) and Art. 9(3).
    ///
    /// Opaque. See the type's note.
    pub commission_time_stamp: String,
    /// **(e)** a hash of the version of the passport the proof was generated for.
    pub passport_version_hash: String,
    /// When the registry generated this proof.
    ///
    /// The clock Art. 9(4)'s ninety days runs from, and the reason this is a
    /// field rather than something derived: it is the registry's statement about
    /// its own act, and a receiving node's own clock is not evidence of it.
    pub generated_at: DateTime<Utc>,
    /// The qualified electronic seal guaranteeing the proof — Art. 9(3).
    ///
    /// Opaque. See the type's note.
    pub qualified_seal: String,
}

impl ProofOfRegistration {
    /// When the registry stops making this proof available — Art. 9(4).
    ///
    /// Ninety **calendar** days from generation. Computed on a UTC instant,
    /// where a day is exactly 24 hours and calendar days and 24-hour periods
    /// coincide; the distinction would matter in a zone with daylight saving,
    /// and the field is deliberately not in one.
    #[must_use]
    pub fn available_until(&self) -> DateTime<Utc> {
        self.generated_at + Duration::days(AVAILABILITY_DAYS)
    }

    /// Whether the registry would still serve this proof at `now`.
    ///
    /// 🚨 **Not "is this proof still valid".** A proof already held remains
    /// evidence, and its seal still verifies; what this answers is whether it
    /// could be fetched again. Named for the article's own word — Art. 9(4) says
    /// *"remain available"* — so that a caller reaching for an expiry check has
    /// to notice it is asking a different question.
    #[must_use]
    pub fn is_available_at(&self, now: DateTime<Utc>) -> bool {
        now < self.available_until()
    }

    /// Validate that what arrived carries the Art. 9(2) minimum.
    ///
    /// # Errors
    ///
    /// [`RegistryValidationError::MissingRequiredField`] for any of the points
    /// that cannot be empty, and whatever [`OperatorIdentifier::validate`]
    /// refuses for point (c).
    ///
    /// The commodity code is checked only when present: Art. 9(2)(b) is *"where
    /// relevant"*, so its absence is lawful and its malformation is not.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        for (name, value) in [
            ("productIdentifier", &self.product_identifier),
            ("commissionTimeStamp", &self.commission_time_stamp),
            ("passportVersionHash", &self.passport_version_hash),
            ("qualifiedSeal", &self.qualified_seal),
        ] {
            if value.trim().is_empty() {
                return Err(RegistryValidationError::MissingRequiredField(name.into()));
            }
        }
        self.operator.validate()?;
        if let Some(code) = &self.commodity_code
            && dpp_domain::CommodityCode::parse(code).is_err()
        {
            return Err(RegistryValidationError::InvalidCommodityCode {
                value: code.clone(),
            });
        }
        // Art. 9(2)(d) is the registration of the *latest version at generation
        // time*, so it cannot postdate the generation. A proof claiming
        // otherwise describes an order of events that did not happen, and is the
        // one internal contradiction the five points can express between them.
        if self.registered_at > self.generated_at {
            return Err(RegistryValidationError::MissingRequiredField(
                "generatedAt".into(),
            ));
        }
        Ok(())
    }
}
