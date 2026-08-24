//! [`Granularity`] — the level a passport describes, as fixed by an act.

use serde::{Deserialize, Serialize};

/// The level at which a passport describes a product: a model, a production
/// batch, or one physical unit.
///
/// # Why this lives in the catalog and not in a port
///
/// ESPR Art. 9(2)(d) makes the level a **delegated-act decision** — "whether the
/// digital product passport is to be established at model, batch or item level".
/// It is therefore a property of the instrument that imposes the passport, and
/// every consumer downstream is reading that decision rather than making one.
/// The EU registry is one such consumer, which is why
/// [`RegistrationGranularity`](crate::ports::registry_sync::RegistrationGranularity)
/// exists and converts from this type rather than the other way round.
///
/// No `Default`, deliberately: an act that has not fixed a level is
/// [`Option::None`] on the instrument, not a guess at item level. Defaulting
/// here would silently answer a question only a delegated act can answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Granularity {
    /// One passport covering every unit sharing a model's specifications.
    Model,
    /// One passport covering every unit made in one production run.
    Batch,
    /// One passport per physical unit.
    Item,
}

impl Granularity {
    /// The more granular of two levels.
    ///
    /// The ordering `Model < Batch < Item` is the derived `Ord`, and matches the
    /// rule the EU registry applies: where several levels are indicated for one
    /// product, the most granular wins (IR (EU) 2026/1778 Art. 8(3), and Art.
    /// 8(4)–(5) linking an item registration back up to batch and model).
    ///
    /// This is why applicable instruments fold to a **maximum** rather than
    /// picking one: two acts naming different levels do not conflict, they
    /// compound.
    #[must_use]
    pub fn most_granular(self, other: Self) -> Self {
        self.max(other)
    }
}
