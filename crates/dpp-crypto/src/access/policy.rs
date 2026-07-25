//! Sector access policy types and disclosure-class lookup.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use dpp_domain::{Disclosure, PASSPORT_FIELD_DISCLOSURE, SectorCatalog};

/// Maps JSON field names to their disclosure class.
///
/// Fields not listed fall back to [`Self::default_disclosure`]. Matching is by
/// **normalized leaf key name** (case- and separator-insensitive), so a policy
/// key `disassemblyInstructions` also covers a payload key
/// `disassembly_instructions` — closing the casing/nesting drift that let
/// restricted fields leak to the public audience.
///
/// **Caution — leaf matching is path-insensitive.** A policy key matches that
/// leaf *wherever* it appears, at any depth. Do **not** restrict a generic leaf
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
    /// Map of JSON field name → disclosure class. A listed field is matched
    /// wherever it appears in the document (any nesting depth), by normalized key.
    pub field_disclosure: HashMap<String, Disclosure>,
    /// Class applied to fields **not** listed in `field_disclosure`. Defaults
    /// to `Public` (backward-compatible: only restricted fields need listing).
    /// Set to a non-public class for a true default-deny (fail-closed) policy,
    /// where every public field must be explicitly listed as `Public`.
    #[serde(default = "disclosure_public")]
    pub default_disclosure: Disclosure,
}

fn disclosure_public() -> Disclosure {
    Disclosure::Public
}

/// Whether `a` and `b` are equal for tier-matching purposes once both are
/// normalized — non-alphanumerics (`_`, `-`) dropped, case-folded, so
/// `disassemblyInstructions` == `disassembly_instructions` — without
/// allocating a `String` for either side. [`SectorAccessPolicy::disclosure_for_field`]
/// runs this once per classified field, per document key, at every
/// recursion depth of [`super::filter::filter_by_audience`], so avoiding an
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

/// Universal conformity-evidence fields present on every published passport
/// payload (signatures, audit trails). Folded into each sector's policy so they
/// are not repeated in every manifest.
const COMMON_CONFORMITY: &[&str] = &[
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
    /// are data in the sector manifests (`disclosure`). Returns `None` if
    /// `sector_key` is not in the catalog.
    pub fn from_catalog(catalog: &SectorCatalog, sector_key: &str) -> Option<Self> {
        let descriptor = catalog.get(sector_key)?;
        let mut field_disclosure: HashMap<String, Disclosure> = descriptor.disclosure.clone();
        for field in COMMON_CONFORMITY {
            field_disclosure
                .entry((*field).to_owned())
                .or_insert(Disclosure::Conformity);
        }
        Some(Self {
            name: format!("{sector_key}-{}", descriptor.current_schema_version),
            sector: sector_key.to_owned(),
            field_disclosure,
            default_disclosure: Disclosure::Public,
        })
    }

    /// Default access policy for top-level passport fields (sector-agnostic).
    ///
    /// **Invariant — no mutable-after-publish *compliance content* may sit at
    /// `Public`.** The public view is what a passport's public signature is
    /// computed over, so `Public` content that changes after publish makes the
    /// served body stop verifying against its own signature. Content that must
    /// stay re-writable post-publish is therefore classified *up*, out of the signed
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
        let mut field_disclosure = HashMap::new();
        for (field, class) in PASSPORT_FIELD_DISCLOSURE {
            field_disclosure.insert((*field).to_owned(), *class);
        }
        Self {
            name: "passport-v1.0".into(),
            sector: "passport".into(),
            field_disclosure,
            default_disclosure: Disclosure::Public,
        }
    }

    /// Get the disclosure class for a field, matched by normalized key name
    /// (case/separator-insensitive). Unlisted fields fall back to
    /// `default_disclosure`.
    pub fn disclosure_for_field(&self, field_name: &str) -> Disclosure {
        self.field_disclosure
            .iter()
            .find(|(k, _)| keys_match_normalized(k, field_name))
            .map(|(_, d)| *d)
            .unwrap_or(self.default_disclosure)
    }
}
