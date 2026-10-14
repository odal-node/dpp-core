//! [`PassportResult`] — what the registry reports about one passport in a submission.

use serde::{Deserialize, Serialize};

/// What the registry reports about one passport of a completed submission.
///
/// 👁️ **The shape of the report is observed; the field names are ours.** User
/// Guide v1.02 chapter 6.2.2 (unchanged in v1.03, 2026-09-16) describes the
/// detail of a completed request: for a `SUCCESS`, the UPI, the URI (the
/// *Unique Registration Identifier*) and the product group of each passport;
/// for a `FAILURE`, the UPI, the product group and the specific error of each
/// passport that failed. The guide describes the web interface and publishes no
/// response schema, so how the API spells these is unverified.
///
/// # Keyed by the identifier, not by position
///
/// A caller matches a result to the passport it sent by
/// [`unique_product_identifier`](Self::unique_product_identifier), which is
/// what the registry reports against. Nothing published says results come back
/// in submission order, and a failure report lists only the passports that
/// failed, so a position would point at the wrong passport as soon as one
/// succeeded and another did not.
///
/// # Both outcomes are optional, because this is read, not built
///
/// A result normally carries exactly one of
/// [`registration_identifier`](Self::registration_identifier) and
/// [`error`](Self::error). The type does not enforce that: it describes what
/// arrived, and a reply carrying neither or both is one the caller should see
/// as it is rather than one this crate refuses to read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportResult {
    /// The unique product identifier the result is about — the URL the
    /// passport was submitted under.
    pub unique_product_identifier: String,
    /// The product group the registry associates with the passport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_group: Option<String>,
    /// The registration identifier the registry generated for the passport.
    ///
    /// ✅ COMPLIANCE-PIN: IR (EU) 2026/1778 **Art. 8(8)** has the registry
    /// generate it following successful verification, and **Art. 8(10)** has
    /// the Commission communicate it *"for that specific product"* through the
    /// API response. `None` for a passport that was not registered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_identifier: Option<String>,
    /// The registry's description of why this passport failed.
    ///
    /// Free text: the guide lists the messages the web interface shows, and a
    /// caller must not match on their wording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
