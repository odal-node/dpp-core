//! [`BundleOrigin`] — the signed bundle a scheduled date came from.

use serde::{Deserialize, Serialize};

/// The signed ruleset bundle a scheduled date came from.
///
/// Both fields travel together because either alone is weak: `bundle_version` is
/// a label the publisher chose and can reuse, and a content hash without a
/// version does not say which bundle an auditor should go and fetch. A receipt
/// of a calculation records the same two fields for the same reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleOrigin {
    /// The bundle's `bundleVersion`, e.g. `"2026-Q3.1"`.
    pub bundle_version: String,
    /// The manifest's `contentSha256`: the bytes the publisher signed over.
    pub content_sha256: String,
}
