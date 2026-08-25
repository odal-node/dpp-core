//! [`PassportView`] — an audience-filtered, serialisable view of a passport.

/// An audience-filtered, serialisable view of a
/// [`Passport`](crate::domain::passport::Passport).
///
/// Produced by [`Passport::redact`](crate::domain::passport::Passport::redact).
/// Serialises transparently to JSON — use this type wherever a consumer
/// should only see the fields allowed by their
/// [`Audience`](crate::domain::identity::Audience).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(transparent)]
pub struct PassportView(pub serde_json::Value);

impl PassportView {
    /// Consume the view and return the underlying JSON value.
    pub fn into_value(self) -> serde_json::Value {
        self.0
    }
}
