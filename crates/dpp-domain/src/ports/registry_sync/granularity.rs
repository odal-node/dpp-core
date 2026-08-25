//! [`RegistrationGranularity`] — the level a passport is registered at.

use serde::{Deserialize, Serialize};

/// The level a passport is registered at, mirrored in the domain so the port
/// does not depend on the registry wire crate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistrationGranularity {
    /// One registration covering every item sharing a product's specifications.
    Model,
    /// One registration covering every item made in one production run.
    Batch,
    /// One registration per physical unit.
    #[default]
    Item,
}

impl From<crate::catalog::Granularity> for RegistrationGranularity {
    /// The registry registers at the level the applicable act fixed.
    ///
    /// Direction matters: [`Granularity`](crate::catalog::Granularity) is the
    /// act's decision under ESPR Art. 9(2)(d), and this type is one consumer of
    /// it. A conversion the other way would let a registry default answer a
    /// question only a delegated act can answer, which is why none exists — and
    /// why this type's `#[default]` of `Item` must never travel back up into the
    /// catalog.
    fn from(granularity: crate::catalog::Granularity) -> Self {
        match granularity {
            crate::catalog::Granularity::Model => Self::Model,
            crate::catalog::Granularity::Batch => Self::Batch,
            crate::catalog::Granularity::Item => Self::Item,
        }
    }
}
