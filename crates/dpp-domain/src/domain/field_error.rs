//! Field-level validation error types.
//!
//! Kept free of any schema/`jsonschema` dependency (unlike the rest of
//! [`crate::domain::validation`], which is wasm-gated) so that
//! [`crate::domain::error::DppError`] can carry structured validation detail on
//! every target, including `wasm32`.

/// A single field-level validation failure.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldError {
    /// JSON pointer path to the failing field, e.g. `"/gtin"` or
    /// `"/fibreComposition/0/pct"`. Empty when the failure is not tied to a
    /// specific field.
    pub field: String,
    /// Human-readable error message.
    pub message: String,
}

/// Collection of field-level errors returned when validation fails.
///
/// Always contains at least one entry.
#[derive(Debug, Clone)]
pub struct ValidationErrors {
    pub errors: Vec<FieldError>,
}

impl ValidationErrors {
    /// A validation failure carrying a single, field-less message.
    pub fn message(msg: impl Into<String>) -> Self {
        Self {
            errors: vec![FieldError {
                field: String::new(),
                message: msg.into(),
            }],
        }
    }

    /// Returns a combined error message listing all failures.
    pub fn to_display(&self) -> String {
        self.errors
            .iter()
            .map(|e| {
                if e.field.is_empty() {
                    e.message.clone()
                } else {
                    format!("{}: {}", e.field, e.message)
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display())
    }
}

impl std::error::Error for ValidationErrors {}

impl From<String> for ValidationErrors {
    fn from(msg: String) -> Self {
        Self::message(msg)
    }
}

impl From<&str> for ValidationErrors {
    fn from(msg: &str) -> Self {
        Self::message(msg)
    }
}
