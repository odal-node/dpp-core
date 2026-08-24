//! [`Instrument`] — one EU legal act, as a catalog record.

use serde::{Deserialize, Serialize};

use super::binding::InstrumentBinding;
use super::granularity::Granularity;
use super::instrument_kind::InstrumentKind;
use super::instrument_status::InstrumentStatus;
use super::passport_obligation::PassportObligation;
use super::retention::RetentionBasis;

/// One EU legal act, with the product groups it reaches.
///
/// # Why an act is a record in its own right
///
/// Its predecessor was an enum variant hanging off a product group, which makes
/// an act's existence conditional on some group naming it. Two facts break that
/// arrangement:
///
/// - one act may reach **many** product groups (a horizontal ecodesign act is
///   designed to), and one group may be reached by **many** acts, so the
///   relation is many-to-many and belongs in neither record alone; and
/// - an act may reach a product group that **does not exist in our catalog** —
///   the Commission's preparatory analysis says the horizontal requirements
///   cover sets of products that were never shortlisted as product groups.
///
/// An instrument must therefore be expressible before, and independently of,
/// any product group that references it.
///
/// # The consequence for callers
///
/// There is no total function from a product group to its instruments, so the
/// applicable set is **recorded, never computed**. What this catalog offers is a
/// lookup over what has been recorded — not a derivation, and never a guarantee
/// of completeness for a product whose set was recorded elsewhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    /// Canonical instrument id, e.g. `"battery-reg-2023-1542"`. Stable across
    /// amendments: an amending act is recorded in `notes` and in the amended
    /// instrument's `legalBasis`, not as a second instrument, because operators
    /// comply with the consolidated act rather than with each amendment.
    pub id: String,
    /// Human-readable title, close to the act's own.
    pub title: String,
    /// CELEX number of the act, e.g. `"32023R1542"` — the anchor a reader uses
    /// to fetch the primary text from EUR-Lex and check every claim in this
    /// record against it. `None` only for [`InstrumentStatus::Anticipated`],
    /// which by definition has no text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub celex: Option<String>,
    /// Whether this is a framework, a direct instrument, or an adjacent act.
    pub kind: InstrumentKind,
    /// How far through the legislative process the act is.
    pub status: InstrumentStatus,
    /// Whether the act requires a passport, and if not, why not. A binding may
    /// override this per product group.
    pub passport: PassportObligation,
    /// Default retention period in years, where the act sets one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_years: Option<u32>,
    /// Whether [`Self::retention_years`] traces to an adopted text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_years_basis: Option<RetentionBasis>,
    /// The level the act fixes, where it fixes one. `None` is the honest answer
    /// for every ESPR product group today: ESPR Art. 9(2)(d) delegates the
    /// choice to acts that do not exist yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granularity: Option<Granularity>,
    /// The framework this act was adopted under, by id — for a delegated or
    /// implementing act. `None` for a framework or a direct instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Product groups this act reaches, and the terms of each reach.
    #[serde(default)]
    pub product_groups: Vec<InstrumentBinding>,
    /// Scope, caveats, amendment history, and the citations behind the fields
    /// above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl Instrument {
    /// This act's binding for a product group, if it reaches it.
    #[must_use]
    pub fn binding(&self, product_group: &str) -> Option<&InstrumentBinding> {
        self.product_groups
            .iter()
            .find(|b| b.product_group == product_group)
    }

    /// Whether this act requires a passport for `product_group`, resolving the
    /// binding's override against [`Self::passport`]. `false` where the act does
    /// not reach the group at all.
    #[must_use]
    pub fn requires_passport_for(&self, product_group: &str) -> bool {
        self.binding(product_group)
            .is_some_and(|b| b.requires_passport(&self.passport))
    }

    /// The passport obligation in force for `product_group` — the binding's
    /// override where it has one, otherwise the act's own.
    #[must_use]
    pub fn passport_for(&self, product_group: &str) -> Option<&PassportObligation> {
        self.binding(product_group)
            .map(|b| b.passport.as_ref().unwrap_or(&self.passport))
    }

    /// Retention this act imposes on `product_group`, with its provenance —
    /// the binding's figure where it has one, otherwise the act's own.
    #[must_use]
    pub fn retention_for(&self, product_group: &str) -> Option<(u32, RetentionBasis)> {
        let binding = self.binding(product_group)?;
        let years = binding.retention_years.or(self.retention_years)?;
        let basis = if binding.retention_years.is_some() {
            binding.retention_years_basis
        } else {
            self.retention_years_basis
        };
        // An unmarked figure is treated as assumed rather than sourced: the safe
        // direction for a claim about someone else's legal obligation.
        Some((years, basis.unwrap_or(RetentionBasis::Assumed)))
    }

    /// The level this act fixes for `product_group` — the binding's where it has
    /// one, otherwise the act's own, and `None` where no act has fixed a level.
    #[must_use]
    pub fn granularity_for(&self, product_group: &str) -> Option<Granularity> {
        let binding = self.binding(product_group)?;
        binding.granularity.or(self.granularity)
    }
}
