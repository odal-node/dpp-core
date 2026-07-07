//! Runtime-registered sector validator trait and registry — the extensibility
//! seam for sectors not known to this crate at compile time.

use crate::domain::field_error::FieldError;

/// Trait for runtime-registered sector validators.
///
/// Register an implementation in [`SectorValidatorRegistry`] to provide JSON
/// Schema + cross-field validation for sectors that are not known to this crate
/// at compile time (e.g., plugin-defined sectors carrying `SectorData::Other`).
pub trait SectorValidator: Send + Sync {
    /// Validate the sector payload (the inner data, without the `"sector"` tag key).
    fn validate(&self, data: &serde_json::Value) -> Result<(), Vec<FieldError>>;
}

/// Registry of runtime sector validators, keyed by catalog sector key.
///
/// An empty registry (the default) causes `SectorData::Other` to fail
/// validation with an "unknown sector" error — silent pass-through is not safe.
#[derive(Default)]
pub struct SectorValidatorRegistry {
    validators: std::collections::HashMap<String, std::sync::Arc<dyn SectorValidator>>,
}

impl SectorValidatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        key: impl Into<String>,
        validator: std::sync::Arc<dyn SectorValidator>,
    ) {
        self.validators.insert(key.into(), validator);
    }

    pub(super) fn get(&self, key: &str) -> Option<&dyn SectorValidator> {
        self.validators.get(key).map(std::sync::Arc::as_ref)
    }
}
