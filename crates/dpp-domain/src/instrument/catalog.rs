//! [`InstrumentCatalog`] — the open, data-driven catalog of legal instruments.

use super::act::Instrument;
use super::binding::InstrumentBinding;
use super::obligation::ObligationDate;
use super::reference::InstrumentRef;
use crate::catalog::error::CatalogError;
use crate::catalog::granularity::Granularity;
use crate::catalog::retention::RetentionBasis;

struct EmbeddedInstrument {
    id: &'static str,
    json: &'static str,
}

/// One manifest per instrument. Adding an act at compile time is a single entry
/// plus a JSON file; adding one at runtime is [`InstrumentCatalog::register`].
const EMBEDDED: &[EmbeddedInstrument] = &[
    EmbeddedInstrument {
        id: "espr",
        json: include_str!("../../instruments/espr.json"),
    },
    EmbeddedInstrument {
        id: "espr-horizontal-repairability",
        json: include_str!("../../instruments/espr-horizontal-repairability.json"),
    },
    EmbeddedInstrument {
        id: "espr-horizontal-eee-recyclability",
        json: include_str!("../../instruments/espr-horizontal-eee-recyclability.json"),
    },
    EmbeddedInstrument {
        id: "battery-reg-2023-1542",
        json: include_str!("../../instruments/battery-reg-2023-1542.json"),
    },
    EmbeddedInstrument {
        id: "toy-safety-2025-2509",
        json: include_str!("../../instruments/toy-safety-2025-2509.json"),
    },
    EmbeddedInstrument {
        id: "detergents-2026-405",
        json: include_str!("../../instruments/detergents-2026-405.json"),
    },
    EmbeddedInstrument {
        id: "cpr-2024-3110",
        json: include_str!("../../instruments/cpr-2024-3110.json"),
    },
    EmbeddedInstrument {
        id: "elv-2026-1738",
        json: include_str!("../../instruments/elv-2026-1738.json"),
    },
    EmbeddedInstrument {
        id: "ecodesign-energy-labelling-mobile",
        json: include_str!("../../instruments/ecodesign-energy-labelling-mobile.json"),
    },
    EmbeddedInstrument {
        id: "ppwr-2025-40",
        json: include_str!("../../instruments/ppwr-2025-40.json"),
    },
    EmbeddedInstrument {
        id: "unsold-goods-format-2026-2",
        json: include_str!("../../instruments/unsold-goods-format-2026-2.json"),
    },
    EmbeddedInstrument {
        id: "unsold-goods-derogations-2026-296",
        json: include_str!("../../instruments/unsold-goods-derogations-2026-296.json"),
    },
];

/// How many instrument manifests ship embedded in this build.
///
/// Exposed so a test can assert the catalog loaded all of them without writing
/// the number down twice.
pub const EMBEDDED_COUNT: usize = EMBEDDED.len();

/// Open, data-driven catalog of the legal instruments that reach our product
/// groups, pre-loaded from embedded manifests and extensible at runtime.
///
/// # What this catalog is not
///
/// It is **not** a derivation of applicable law. There is no total function from
/// a product group to the acts that reach it — a horizontal act may cover
/// products that were never shortlisted as product groups — so what a passport
/// records as its applicable set is recorded at issuance, not looked up here
/// afterwards. This catalog answers "what have we recorded about this act", and
/// its folds answer "given the acts we know of, what do they compound to". A
/// caller must not read a fold as a statement that nothing else applies.
///
/// # Status
///
/// Wired and authoritative for law. Status, legal basis, passport obligation,
/// dates, retention and granularity left
/// [`ProductGroupDescriptor`](crate::catalog::ProductGroupDescriptor) entirely, so there
/// is no second copy to drift from — a question about what binds a product group
/// has exactly one place to be asked.
pub struct InstrumentCatalog {
    entries: Vec<Instrument>,
}

impl InstrumentCatalog {
    /// Create a catalog pre-loaded with all embedded instrument manifests.
    ///
    /// # Panics
    /// If an embedded manifest is malformed or its `id` does not match its
    /// filename — both are build-time authoring errors in this crate.
    #[must_use]
    pub fn new() -> Self {
        let entries = EMBEDDED
            .iter()
            .map(|m| {
                let instrument: Instrument = serde_json::from_str(m.json).unwrap_or_else(|e| {
                    panic!("embedded instrument manifest '{}' is invalid: {e}", m.id)
                });
                assert_eq!(
                    instrument.id, m.id,
                    "manifest id '{}' does not match its file id '{}'",
                    instrument.id, m.id
                );
                instrument
            })
            .collect();
        Self { entries }
    }

    /// Look up an instrument by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Instrument> {
        self.entries.iter().find(|i| i.id == id)
    }

    /// All instruments.
    #[must_use]
    pub fn all(&self) -> &[Instrument] {
        &self.entries
    }

    /// Every recorded (instrument, binding) pair reaching `product_group`.
    ///
    /// The pair rather than the instrument alone, because the binding carries
    /// the terms: two groups reached by one act can differ in status, dates,
    /// retention and level.
    #[must_use]
    pub fn bindings_for(&self, product_group: &str) -> Vec<(&Instrument, &InstrumentBinding)> {
        self.entries
            .iter()
            .filter_map(|i| i.binding(product_group).map(|b| (i, b)))
            .collect()
    }

    /// The pairs under which a binding compliance determination may be made for
    /// `product_group`.
    ///
    /// Returns the pairs rather than a boolean **deliberately**. A determination
    /// is always made under a named act, and a caller that only learns "yes"
    /// cannot say which act it is asserting against — which is how a
    /// determination came to be emitted against an obligation that did not
    /// exist. Callers should pass the instrument through to whatever records the
    /// result.
    #[must_use]
    pub fn determinable_for(&self, product_group: &str) -> Vec<(&Instrument, &InstrumentBinding)> {
        self.bindings_for(product_group)
            .into_iter()
            .filter(|(_, b)| b.allows_determination())
            .collect()
    }

    /// Whether any recorded act requires a passport for `product_group`.
    ///
    /// Independent of [`Self::determinable_for`]: an act may bind today and
    /// require no passport (unsold goods), or require a passport whose date has
    /// not arrived (batteries).
    #[must_use]
    pub fn passport_required_for(&self, product_group: &str) -> bool {
        self.entries
            .iter()
            .any(|i| i.requires_passport_for(product_group))
    }

    /// The **earliest** date at which any recorded act requires a passport for
    /// `product_group`.
    ///
    /// Earliest because the obligations accumulate: once the first act's date
    /// arrives, a passport is owed, whatever the others say. `None` where no
    /// recorded act requires one, or where none has fixed a date.
    ///
    /// Dates are compared as ISO-8601 strings, which orders correctly for
    /// `YYYY-MM-DD` and is why the format is fixed by the field's contract.
    #[must_use]
    pub fn passport_due_for(&self, product_group: &str) -> Option<&ObligationDate> {
        self.entries
            .iter()
            .filter_map(|i| i.passport_for(product_group)?.applies_from())
            .min_by(|a, b| a.date.cmp(&b.date))
    }

    /// The retention period `product_group` must satisfy across every recorded
    /// act, with the provenance of that figure.
    ///
    /// The **maximum**, because retention periods are floors and a record kept
    /// long enough for the longest satisfies them all. The basis is
    /// [`RetentionBasis::Sourced`] only when *every* contributing figure is
    /// sourced — one assumption anywhere makes the compound figure an
    /// assumption, whichever act happened to supply the maximum.
    #[must_use]
    pub fn retention_for(&self, product_group: &str) -> Option<(u32, RetentionBasis)> {
        let figures: Vec<(u32, RetentionBasis)> = self
            .entries
            .iter()
            .filter_map(|i| i.retention_for(product_group))
            .collect();
        let years = figures.iter().map(|(y, _)| *y).max()?;
        let basis = if figures.iter().all(|(_, b)| *b == RetentionBasis::Sourced) {
            RetentionBasis::Sourced
        } else {
            RetentionBasis::Assumed
        };
        Some((years, basis))
    }

    /// The most granular level any recorded act fixes for `product_group`.
    ///
    /// Most granular because levels compound rather than conflict — an item-level
    /// record satisfies a model-level requirement, and the EU registry links an
    /// item registration back up to batch and model rather than treating them as
    /// alternatives. `None` where no recorded act has fixed a level, which is the
    /// position of every ESPR product group today.
    #[must_use]
    pub fn granularity_for(&self, product_group: &str) -> Option<Granularity> {
        self.entries
            .iter()
            .filter_map(|i| i.granularity_for(product_group))
            .reduce(Granularity::most_granular)
    }

    /// The instrument references to record on a passport being issued for
    /// `product_group` at `at`.
    ///
    /// This is the **only** moment the catalog is consulted for a passport's
    /// applicable set: what it returns is written onto the record and never
    /// recomputed. See [`InstrumentRef`] for why re-deriving it later would be
    /// both legally wrong and lossy.
    ///
    /// Returns every act reaching the group, whatever its status — a passport
    /// issued today for a group whose act applies in 2027 is still governed by
    /// that act, and dropping it here would leave the record unable to say so.
    /// Filtering by status is a question for whoever renders or determines, and
    /// they have the binding to filter with.
    ///
    /// ⚠️ **Not exhaustive, and callers must not treat it as such.** An act may
    /// apply to a product while reaching no product group we model, so an
    /// operator-supplied set is merged with this one rather than validated
    /// against it.
    #[must_use]
    pub fn instrument_refs_for(&self, product_group: &str) -> Vec<InstrumentRef> {
        self.bindings_for(product_group)
            .into_iter()
            .map(|(instrument, _)| InstrumentRef::from_catalog(instrument.id.clone()))
            .collect()
    }

    /// Every product-group key any recorded act reaches, sorted and deduplicated.
    ///
    /// Includes keys with no entry in the product-group catalog — the horizontal
    /// case this catalog exists to represent. Callers rendering these to a user
    /// must resolve each against the product-group catalog and handle absence,
    /// rather than assuming a descriptor exists.
    #[must_use]
    pub fn product_group_keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self
            .entries
            .iter()
            .flat_map(|i| i.product_groups.iter())
            .map(|b| b.product_group.as_str())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        keys
    }

    /// All instrument ids, sorted.
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.entries.iter().map(|i| i.id.as_str()).collect();
        ids.sort_unstable();
        ids
    }

    /// Register an instrument at runtime.
    ///
    /// # Errors
    /// [`CatalogError::AlreadyExists`] if the id is already taken.
    pub fn register(&mut self, instrument: Instrument) -> Result<(), CatalogError> {
        if self.get(&instrument.id).is_some() {
            return Err(CatalogError::AlreadyExists(instrument.id));
        }
        self.entries.push(instrument);
        Ok(())
    }

    /// Number of instruments in the catalog.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the catalog is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for InstrumentCatalog {
    fn default() -> Self {
        Self::new()
    }
}
