//! [`PassportId`] — the passport's unique identifier.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Newtype wrapper for a passport's unique identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PassportId(pub Uuid);

impl PassportId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for PassportId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PassportId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
