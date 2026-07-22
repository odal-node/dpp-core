//! Sector access policy types and tier lookup.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use dpp_domain::{AccessTier, SectorCatalog};

/// Maps JSON field names to their minimum access tier.
///
/// Fields not listed fall back to [`Self::default_tier`]. Matching is by
/// **normalized leaf key name** (case- and separator-insensitive), so a policy
/// key `disassemblyInstructions` also covers a payload key
/// `disassembly_instructions` — closing the casing/nesting drift that let
/// elevated fields leak at the Public tier (crypto Gap 6).
///
/// **Caution — leaf matching is path-insensitive.** A policy key matches that
/// leaf *wherever* it appears, at any depth. Do **not** elevate a generic leaf
/// name shared across objects (e.g. `name`, `value`, `country`, `address`): such
/// a key would redact `facility.address` *and* `manufacturer.address` alike,
/// over-redacting Annex III public fields. Use only specific, unambiguous field
/// names (e.g. `dueDiligenceUrl`, `svhcSubstances`). Gating a shared leaf on a
/// single path would require making the matcher path-aware first.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorAccessPolicy {
    /// Human-readable policy name (e.g., `"textile-v1.1"`).
    pub name: String,
    /// The sector this policy applies to.
    pub sector: String,
    /// Map of JSON field name → minimum access tier. A listed field is matched
    /// wherever it appears in the document (any nesting depth), by normalized key.
    pub field_tiers: HashMap<String, AccessTier>,
    /// Tier applied to fields **not** listed in `field_tiers`. Defaults to
    /// `Public` (backward-compatible: only elevated fields need listing). Set to
    /// `Confidential` for a true default-deny (fail-closed) policy, where every
    /// public field must be explicitly listed as `Public`.
    #[serde(default = "tier_public")]
    pub default_tier: AccessTier,
}

fn tier_public() -> AccessTier {
    AccessTier::Public
}

/// Whether `a` and `b` are equal for tier-matching purposes once both are
/// normalized — non-alphanumerics (`_`, `-`) dropped, case-folded, so
/// `disassemblyInstructions` == `disassembly_instructions` — without
/// allocating a `String` for either side. [`SectorAccessPolicy::tier_for_field`]
/// runs this once per policy-tiered field, per document key, at every
/// recursion depth of [`super::filter::filter_by_access_tier`], so avoiding an
/// allocation per comparison matters there.
fn keys_match_normalized(a: &str, b: &str) -> bool {
    let mut a_chars = a.chars().filter(char::is_ascii_alphanumeric);
    let mut b_chars = b.chars().filter(char::is_ascii_alphanumeric);
    loop {
        match (a_chars.next(), b_chars.next()) {
            (Some(x), Some(y)) if x.eq_ignore_ascii_case(&y) => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

/// Universal confidential fields present on every published passport payload
/// (signatures, audit trails). Folded into each sector's policy so they are not
/// repeated in every manifest.
const COMMON_CONFIDENTIAL: &[&str] = &[
    "jwsSignature",
    "complianceReport",
    "auditHistory",
    "supplyChainTrace",
];

impl SectorAccessPolicy {
    /// Build a sector's access policy from the catalog's declared per-field
    /// tiers, folding in the universal confidential fields.
    ///
    /// This works for **every** sector with no per-sector Rust code — the tiers
    /// are data in the sector manifests (`access_tiers`). Returns `None` if
    /// `sector_key` is not in the catalog.
    pub fn from_catalog(catalog: &SectorCatalog, sector_key: &str) -> Option<Self> {
        let descriptor = catalog.get(sector_key)?;
        let mut field_tiers: HashMap<String, AccessTier> = descriptor.access_tiers.clone();
        for field in COMMON_CONFIDENTIAL {
            field_tiers
                .entry((*field).to_owned())
                .or_insert(AccessTier::Confidential);
        }
        Some(Self {
            name: format!("{sector_key}-{}", descriptor.current_schema_version),
            sector: sector_key.to_owned(),
            field_tiers,
            default_tier: AccessTier::Public,
        })
    }

    /// Default access policy for top-level passport fields (sector-agnostic).
    ///
    /// **Invariant — no mutable-after-publish *compliance content* may sit at
    /// `Public`.** The public view is what a passport's public signature is
    /// computed over, so `Public` content that changes after publish makes the
    /// served body stop verifying against its own signature. Content that must
    /// stay re-writable post-publish is therefore tiered *up*, out of the signed
    /// public payload — see `lintResult` below.
    ///
    /// **The exemption, stated so it is not read as an oversight.** Lifecycle
    /// metadata — `status`, `publishedAt`, `updatedAt`, `qrCodeUrl` — is `Public`
    /// *and* mutable after publish. That is consistent only because a conforming
    /// server serves the **signed payload**, not the live row: what it emits is
    /// frozen at publish time and therefore agrees with the attached signature by
    /// construction. A server that redacts the live row into a public view and
    /// attaches the publish-time proof to it reintroduces exactly the divergence
    /// this invariant exists to prevent, for these fields and any future one.
    pub fn passport_default() -> Self {
        let mut field_tiers = HashMap::new();

        // Professional tier
        field_tiers.insert("batchId".into(), AccessTier::Professional);
        // `lintResult` is advisory plausibility output that is deliberately
        // re-computable at any time (including after publish), and every re-run
        // restamps `assessedAt`. Keeping it Public would put a guaranteed-to-
        // change field inside the signed public view. It is also operator- and
        // auditor-facing QA data — the findings carry free-text messages about
        // *our own* data quality — which is not consumer-facing content.
        field_tiers.insert("lintResult".into(), AccessTier::Professional);

        // Confidential tier — signature / internal
        field_tiers.insert("jwsSignature".into(), AccessTier::Confidential);
        field_tiers.insert("retentionLocked".into(), AccessTier::Confidential);

        Self {
            name: "passport-v1.0".into(),
            sector: "passport".into(),
            field_tiers,
            default_tier: AccessTier::Public,
        }
    }

    /// Get the minimum access tier for a field, matched by normalized key name
    /// (case/separator-insensitive). Unlisted fields fall back to `default_tier`.
    pub fn tier_for_field(&self, field_name: &str) -> AccessTier {
        self.field_tiers
            .iter()
            .find(|(k, _)| keys_match_normalized(k, field_name))
            .map(|(_, t)| *t)
            .unwrap_or(self.default_tier)
    }
}
