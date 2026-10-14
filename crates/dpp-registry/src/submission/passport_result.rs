//! [`PassportResult`] — what the registry reports about one passport in a submission.

use serde::{Deserialize, Serialize};

use super::PassportOutcome;

/// What the registry reports about one passport of a completed submission.
///
/// 👁️ **Observed, except the registration identifier.** The registry's web
/// client, read on 2026-10-14, takes each entry of a registration task's
/// `resultPayload.report.itemDetails` as `uniqueProductIdentifier`,
/// `productGroup`, `outcome` and `errorCode`, and those are the names used
/// here. User Guide v1.02 chapter 6.2.2 (unchanged in v1.03) says a successful
/// request also lists each passport's URI (the *Unique Registration
/// Identifier*); the client shows no such column, so
/// [`registration_identifier`](Self::registration_identifier) is named by us.
/// None of this is a published schema.
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
/// # Every outcome is optional, because this is read, not built
///
/// The type does not enforce that a rejected passport carries an error code or
/// an accepted one an identifier: it describes what arrived, and a reply that
/// breaks the pattern is one the caller should see as it is rather than one
/// this crate refuses to read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportResult {
    /// The unique product identifier the result is about — the URL the
    /// passport was submitted under.
    pub unique_product_identifier: String,
    /// The product group the registry associates with the passport, by its
    /// registry code (`BATTERIES`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_group: Option<String>,
    /// What the registry ruled about this passport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<PassportOutcome>,
    /// The registration identifier the registry generated for the passport.
    ///
    /// ✅ COMPLIANCE-PIN: IR (EU) 2026/1778 **Art. 8(8)** has the registry
    /// generate it following successful verification, and **Art. 8(10)** has
    /// the Commission communicate it *"for that specific product"* through the
    /// API response. `None` for a passport that was not registered.
    ///
    /// ⚠️ The field name is ours: no client code reading one was seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_identifier: Option<String>,
    /// Why the registry refused this passport, as a code.
    ///
    /// 👁️ A machine code, not text: the client looks the wording up in its
    /// own translations. The codes seen are listed in
    /// [`REGISTRATION_ERROR_CODES`](crate::REGISTRATION_ERROR_CODES); a
    /// code outside that list is still carried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}
