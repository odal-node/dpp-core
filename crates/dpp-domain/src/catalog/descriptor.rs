//! [`ProductGroupDescriptor`] — a single product group's catalog entry.

use serde::{Deserialize, Serialize};

/// A single product group's catalog entry — the canonical record every
/// component (schema registry, plugin host, passport model) resolves against.
///
/// # What this record is not
///
/// It carries **no law**. `status`, `regime`, `legalBasis`, `dppAppliesFrom`,
/// `retentionYears` and `retentionYearsBasis` were fields here, and every one of
/// them was singular — which asserts that exactly one act governs a product
/// group. ESPR Art. 5(7) says otherwise: one delegated act may cover many
/// product groups, a group-specific act may supplement a horizontal one, and the
/// Regulation contains no precedence rule anywhere, so overlapping acts
/// accumulate. Each of those fields is a property of an *(act, product group)*
/// pair and now lives on
/// [`InstrumentBinding`](crate::instrument::InstrumentBinding), reached through
/// [`InstrumentCatalog`](crate::instrument::InstrumentCatalog).
///
/// What is left is identity, scope, and our own implementation of it: what the
/// group is called, what sub-types it has, which schema versions we serve for
/// it, how its fields are disclosed, and which plugin handles it. None of that
/// changes when a new act arrives; all of the law does.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductGroupDescriptor {
    /// Canonical product-group key, e.g. `"battery"`, `"unsold-goods"`. Matches
    /// the schema-registry key and the plugin's `meta().product_group`.
    pub key: String,
    /// Human-readable title.
    pub title: String,
    /// Schema versions available for this product group (semver strings).
    pub schema_versions: Vec<String>,
    /// The schema version applicable to *new* passports in this product group
    /// right now. Decouples "current" from "latest embedded" so a future schema
    /// can ship embedded without becoming current until its act is in force.
    /// Must be one of `schema_versions`.
    pub current_schema_version: String,
    /// Product categories *within* this product group — sub-types a plugin may
    /// branch on, never dispatch keys. See `DATA-MODEL.md` §3.4.
    #[serde(default)]
    pub product_categories: Vec<String>,
    /// Per-field [`Disclosure`](crate::disclosure::Disclosure) class for
    /// this product group's data: field name → class; unlisted fields default to
    /// public. Not an ordering — a class names which audiences may see the
    /// field, and the audiences do not nest. Universal conformity fields
    /// (signatures, audit trails) are folded in by the access-policy engine, so
    /// they are not repeated per product group here.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub disclosure: std::collections::HashMap<String, crate::disclosure::Disclosure>,
    /// Plugin that handles this product group (crate / filename stem, e.g.
    /// `"product-group-battery"`). `None` if no plugin is bound yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// Free-text note about scope or implementation. Regulatory notes belong on
    /// the instrument or its binding, with the act they describe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}
