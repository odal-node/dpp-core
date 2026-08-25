//! `PluginHost` port — delegates compliance computation to loaded Wasm product group plugins.

use crate::domain::compliance::{ComplianceError, ComplianceResult};
use crate::domain::product_group::ProductGroupData;

/// Port trait for the Wasm plugin host.
///
/// Implementors load product group-specific Wasm plugins and delegate compliance
/// computation to them. Tests can wire a stub that returns fixed results.
/// The passthrough registry is used when no plugin is loaded for a product group.
///
/// Dispatch is by **catalog key**, not by the `ProductGroup` enum. Plugin manifests
/// are string-keyed and the catalog is the source of product group identity; taking
/// the enum here meant a plugin could only ever be loaded for a product group this
/// build already had a variant for, which is the opposite of what a plugin
/// host is for. Use [`ProductGroup::catalog_key`](crate::domain::product_group::ProductGroup::catalog_key)
/// at the call site.
pub trait PluginHost: Send + Sync {
    /// Returns true if a Wasm plugin is currently loaded for `product_group_key`.
    fn has_plugin(&self, product_group_key: &str) -> bool;

    /// Invoke the loaded plugin for `product_group_key` with the given `data`.
    ///
    /// Returns `ComplianceErrorKind::UnknownProductGroup` if no plugin is loaded.
    fn compute(
        &self,
        product_group_key: &str,
        data: &ProductGroupData,
    ) -> Result<ComplianceResult, ComplianceError>;
}
