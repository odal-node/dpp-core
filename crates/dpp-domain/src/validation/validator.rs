//! Runtime-registered product group validator trait and registry — the extensibility
//! seam for product groups not known to this crate at compile time.

use crate::field_error::FieldError;

/// Trait for runtime-registered product group validators.
///
/// Register an implementation in [`ProductGroupValidatorRegistry`] to provide JSON
/// Schema + cross-field validation for product groups that are not known to this crate
/// at compile time (e.g., plugin-defined product groups carrying `ProductGroupData::Other`).
pub trait ProductGroupValidator: Send + Sync {
    /// Validate the product group payload (the inner data, without the `"productGroup"` tag key).
    fn validate(&self, data: &serde_json::Value) -> Result<(), Vec<FieldError>>;
}

/// Registry of runtime product group validators, keyed by catalog product group key.
///
/// An empty registry (the default) causes `ProductGroupData::Other` to fail
/// validation with an "unknown product group" error — silent pass-through is not safe.
#[derive(Default)]
pub struct ProductGroupValidatorRegistry {
    validators: std::collections::HashMap<String, std::sync::Arc<dyn ProductGroupValidator>>,
}

impl ProductGroupValidatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        key: impl Into<String>,
        validator: std::sync::Arc<dyn ProductGroupValidator>,
    ) {
        self.validators.insert(key.into(), validator);
    }

    pub(super) fn get(&self, key: &str) -> Option<&dyn ProductGroupValidator> {
        self.validators.get(key).map(std::sync::Arc::as_ref)
    }
}
