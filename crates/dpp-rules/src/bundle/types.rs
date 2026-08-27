//! Wire types for a signed, versioned ruleset bundle.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Signed description of a ruleset bundle — the JWS payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RulesetManifest {
    /// Channel bundle version, e.g. `"2026-Q3.1"`.
    pub bundle_version: String,
    /// When this bundle's rules take effect.
    pub effective_date: DateTime<Utc>,
    /// EU-act citations this bundle encodes (audit trail for the change).
    #[serde(default)]
    pub act_citations: Vec<String>,
    /// ProductGroup → schema version this bundle references (never forks schemas).
    #[serde(default)]
    pub schema_versions: BTreeMap<String, String>,
    /// Hex SHA-256 over the JCS-canonicalised `content`.
    pub content_sha256: String,
}

/// A signed bundle on the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedBundle {
    /// Compact EdDSA JWS over the manifest, signed by the publisher key.
    pub manifest_jws: String,
    /// The ruleset payload the manifest commits to.
    pub content: serde_json::Value,
}

/// Where a ruleset came from.
///
/// A caller that must not act on unverified rules can branch on this. It exists
/// because the alternative — asserting that every [`RulesetAcceptance`] is
/// verified — was not true, and could not be made true without removing a
/// legitimate caller's only way to start up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesetProvenance {
    /// Signature and content hash both checked by
    /// [`crate::bundle::verify_bundle`] against a pinned publisher key.
    Verified,
    /// Compiled-in defaults, adopted without a signature because there is no
    /// channel configured and no bytes arrived from anywhere.
    LocalBaseline,
}

/// A ruleset a caller has accepted, and how it came to be accepted.
///
/// # Why this is not called `VerifiedRuleset` any more
///
/// It was, and its doc said *"Only constructible via `verify_bundle`, so holding
/// one is proof it verified."* Both halves were false: every field was public
/// with no `#[non_exhaustive]`, so anyone could build one, and the one
/// production consumer did — a node with no configured channel seeds itself with
/// a compiled-in baseline that has no signature to check.
///
/// That was not a misuse. A node has to start somewhere, and the baseline is the
/// honest default. What was wrong was a type asserting an invariant it did not
/// hold, while a reader downstream had no way to tell the two cases apart.
///
/// So the fields are private, the two provenances are named, and
/// [`Self::provenance`] answers the question the old doc only claimed to.
#[derive(Debug, Clone)]
pub struct RulesetAcceptance {
    manifest: RulesetManifest,
    content: serde_json::Value,
    provenance: RulesetProvenance,
}

impl RulesetAcceptance {
    /// Adopt a compiled-in baseline ruleset **without verification**.
    ///
    /// For the no-channel default only. The name says `unverified` because a
    /// call site is where this decision is visible, and it should read as a
    /// decision rather than as a constructor.
    #[must_use]
    pub fn unverified_baseline(manifest: RulesetManifest, content: serde_json::Value) -> Self {
        Self {
            manifest,
            content,
            provenance: RulesetProvenance::LocalBaseline,
        }
    }

    /// Construct a verified acceptance. Crate-private: the only way to obtain
    /// one from outside is to pass [`crate::bundle::verify_bundle`].
    pub(crate) fn verified(manifest: RulesetManifest, content: serde_json::Value) -> Self {
        Self {
            manifest,
            content,
            provenance: RulesetProvenance::Verified,
        }
    }

    /// How this ruleset was accepted.
    #[must_use]
    pub fn provenance(&self) -> RulesetProvenance {
        self.provenance
    }

    /// The manifest.
    #[must_use]
    pub fn manifest(&self) -> &RulesetManifest {
        &self.manifest
    }

    /// The ruleset payload.
    #[must_use]
    pub fn content(&self) -> &serde_json::Value {
        &self.content
    }

    /// The active bundle version (surfaced on `/health`, stamped into provenance).
    #[must_use]
    pub fn version(&self) -> &str {
        &self.manifest.bundle_version
    }
}

/// What a caller will accept, beyond a valid signature.
///
/// A signature says a bundle is authentic. It says nothing about whether the
/// bundle is *current* or *yet applicable*, and both were unchecked until
/// 2026-08-27 — so a validly-signed bundle from two years ago, or one whose
/// rules start next year, was adopted the moment it arrived.
#[derive(Debug, Clone, Copy)]
pub struct AcceptancePolicy<'a> {
    /// The caller's clock. Passed in rather than read here so the decision is
    /// reproducible and testable, and because this crate is `no_std` without
    /// the `bundle` feature and has no business owning a clock.
    pub now: DateTime<Utc>,
    /// The effective date of the ruleset currently in force, if any.
    ///
    /// A bundle older than this is refused as superseded. `None` means nothing
    /// is in force yet and any effective bundle is acceptable — correct for a
    /// cold start, and the reason this is an `Option` rather than a sentinel
    /// date.
    pub in_force: Option<&'a RulesetManifest>,
}

/// Why a bundle was refused. Verification is fail-closed — any of these keeps
/// the caller on its current ruleset.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RulesetError {
    /// The manifest JWS did not verify under the pinned publisher key.
    #[error("bundle signature invalid or not signed by the pinned publisher key")]
    BadSignature,
    /// `content` does not hash to the value in the signed manifest.
    #[error("bundle content hash mismatch — content does not match the signed manifest")]
    ContentHashMismatch,
    /// The bundle is authentic but its rules do not take effect yet.
    ///
    /// Not an error in the bundle — a statement about timing. A caller staging
    /// a future ruleset should hold the bytes and re-offer them once `now`
    /// reaches `effective_date`.
    #[error(
        "bundle '{bundle_version}' takes effect at {effective_date} and it is now {now} — \
         not yet applicable"
    )]
    NotYetEffective {
        /// The refused bundle's version.
        bundle_version: String,
        /// When its rules take effect.
        effective_date: DateTime<Utc>,
        /// The clock the caller supplied.
        now: DateTime<Utc>,
    },
    /// The bundle is authentic but older than the ruleset already in force.
    ///
    /// The rollback case: a signature never expires, so anyone able to serve
    /// bytes could otherwise pin a node to superseded rules without forging
    /// anything.
    #[error(
        "bundle '{offered_version}' is effective {offered} but '{in_force_version}' effective \
         {in_force} is already in force — refusing to roll back"
    )]
    Superseded {
        /// The refused bundle's version.
        offered_version: String,
        /// Its effective date.
        offered: DateTime<Utc>,
        /// The version currently in force.
        in_force_version: String,
        /// The effective date currently in force.
        in_force: DateTime<Utc>,
    },
    /// The bundle was structurally malformed.
    #[error("malformed bundle: {0}")]
    Malformed(String),
}
