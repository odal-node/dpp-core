//! [`SectorDescriptor`] — a single sector's catalog entry.

use serde::{Deserialize, Serialize};

use super::status::RegulatoryStatus;

/// A single sector's catalog entry — the canonical record every component
/// (schema registry, plugin host, passport model) resolves against.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorDescriptor {
    /// Canonical sector key, e.g. `"battery"`, `"unsold-goods"`. Matches the
    /// schema-registry sector key and the plugin's `meta().sector`.
    pub key: String,
    /// Human-readable title.
    pub title: String,
    /// Regulatory status — gates whether determinations are binding.
    pub status: RegulatoryStatus,
    /// EU legal instrument(s) this sector derives from.
    pub legal_basis: Vec<String>,
    /// ISO-8601 date the DPP obligation applies from, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dpp_applies_from: Option<String>,
    /// Minimum data retention in years required by the applicable act.
    pub retention_years: u32,
    /// Schema versions available for this sector (semver strings).
    pub schema_versions: Vec<String>,
    /// The schema version applicable to *new* passports in this sector right
    /// now. Decouples "current" from "latest embedded" so a future schema can
    /// ship embedded without becoming current until its act is in force. Must
    /// be one of `schema_versions`.
    pub current_schema_version: String,
    /// Product categories *within* this sector — sub-types a plugin may branch
    /// on, never dispatch keys. See `DATA-MODEL.md` §3.5.
    #[serde(default)]
    pub product_categories: Vec<String>,
    /// Per-field minimum ESPR access tier (public/professional/confidential) for
    /// this sector's data: field name → tier; unlisted fields default to public.
    /// Universal confidential fields (signatures, audit trails) are folded in by
    /// the access-policy engine, so they are not repeated per sector here.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub access_tiers: std::collections::HashMap<String, crate::domain::identity::AccessTier>,
    /// Plugin that handles this sector (crate / filename stem, e.g.
    /// `"sector-battery"`). `None` if no plugin is bound yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// Free-text regulatory note (effective dates, scope, caveats).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}
