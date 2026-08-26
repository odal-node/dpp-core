//! [`DerivedView`] — an upcast view of product group data, with honest provenance.

use serde_json::Value;

/// A derived (upcast) view of product group data, with honest provenance. Never the
/// canonical signed original — `derived` is always `true`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivedView {
    /// The transformed product group data, conforming to the `to` schema.
    pub data: Value,
    /// Always `true`: this is a read-time derivation, not signed source.
    pub derived: bool,
    /// The version derived from, and the version now conformed to.
    pub from: String,
    pub to: String,
    /// The ordered hops applied — `[["1.0.0","2.0.0"]]` — for multi-hop chains.
    pub lens_chain: Vec<[String; 2]>,
    /// `true` if any hop in the chain dropped or defaulted information.
    pub lossy: bool,
}
