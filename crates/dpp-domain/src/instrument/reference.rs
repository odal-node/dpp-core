//! [`InstrumentRef`] — one act recorded on a passport as applicable to it.

use serde::{Deserialize, Serialize};

/// Where a recorded instrument came from.
///
/// Not a confidence ranking — both values are equally authoritative on a
/// passport. What they distinguish is *who asserted it*, which matters when a
/// record is audited years later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RecordedBasis {
    /// Resolved from the instrument catalog at issuance: an act we hold a
    /// manifest for, reaching this product group through a recorded binding.
    Catalog,
    /// Asserted by the economic operator placing the product on the market.
    ///
    /// **Not a fallback.** The catalog cannot be exhaustive: the Commission's
    /// own preparatory analysis says horizontal ecodesign requirements cover
    /// sets of products never shortlisted as product groups, so an act may
    /// apply to a product while reaching no product group we model. An operator
    /// who knows an act applies must be able to say so, and a model that only
    /// accepted catalog-derived entries would force them to leave it out.
    Operator,
}

/// One legal instrument recorded on a passport as applicable to it.
///
/// # Recorded, never recomputed
///
/// The applicable set is fixed when the product is placed on the market —
/// `placedOnMarketDate` on the passport is the same moment — and is never
/// re-derived afterwards. Two reasons, and the second is the load-bearing one:
///
/// 1. **The law that governs a product is the law at placing on the market.**
///    Recomputing later would silently re-govern a published record by acts
///    adopted after it was issued.
/// 2. **There is no function to recompute it with.** Applicable instruments are
///    not derivable from the product group — see [`RecordedBasis::Operator`] —
///    so any "refresh" would quietly *narrow* the set to whatever the catalog
///    happens to know, dropping exactly the entries a human had to supply.
///
/// It is therefore a protected field: not patchable, not in the
/// retention-mutable set, and corrected only by superseding the passport with a
/// new version. A mis-recorded legal basis is a fact about a published record,
/// and the way to fix a published record is to publish a corrected one.
///
/// # Why there is no timestamp here
///
/// An earlier shape carried a per-entry `recordedAt`. It was **redundant** —
/// the whole set is written at issuance, so every entry's timestamp equalled the
/// passport's own `createdAt` — and a redundant copy of a date is a date that
/// can disagree with the one it duplicates.
///
/// Removing it also sidesteps a live hazard worth knowing about: the access
/// filter classifies nested keys **by name, not by path**, using a policy built
/// from the *product group's* schema. The battery schema declares its own
/// `recordedAt` as individual-tier data (Annex XIII point 4), so this field's
/// timestamp inherited that class and was redacted out of every public battery
/// projection — leaving a document that no longer deserialised. Any envelope
/// field whose nested key name collides with a product-group field name will hit
/// the same thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentRef {
    /// The instrument's catalog id, e.g. `"battery-reg-2023-1542"`.
    ///
    /// Free-form rather than validated against the catalog, for the same reason
    /// [`RecordedBasis::Operator`] exists: an act the catalog does not model is
    /// still an act, and refusing to record it would lose the one piece of
    /// information nothing else can supply.
    pub instrument: String,
    /// Who asserted that this act applies.
    pub recorded: RecordedBasis,
}

impl InstrumentRef {
    /// A reference resolved from the instrument catalog.
    #[must_use]
    pub fn from_catalog(instrument: impl Into<String>) -> Self {
        Self {
            instrument: instrument.into(),
            recorded: RecordedBasis::Catalog,
        }
    }

    /// A reference asserted by the economic operator.
    #[must_use]
    pub fn from_operator(instrument: impl Into<String>) -> Self {
        Self {
            instrument: instrument.into(),
            recorded: RecordedBasis::Operator,
        }
    }
}
