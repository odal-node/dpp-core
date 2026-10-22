//! [`ObligationEvent`] — one dated obligation taking effect.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::ObligationKind;
use crate::instrument::DateBasis;

/// One dated obligation taking effect.
///
/// # Read `basis` before presenting `date`
///
/// A [`DateBasis::Assumed`] date is carried as a working assumption, and must
/// never be shown to an operator as a deadline. The basis travels with every
/// event for that reason: a date with no traceable source is indistinguishable
/// from a sourced one unless the record says which it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationEvent {
    /// Stable identifier, so a consumer that receives an event more than once
    /// can recognise it. A rule's change is `<product group>.<subject>`, e.g.
    /// `"battery.due-diligence-report"`; an act's passport start is
    /// `<instrument id>.passport`, followed by `.<product group>` when it is the
    /// date of one product group's binding.
    pub id: String,
    /// The first day the obligation applies.
    pub date: NaiveDate,
    /// What kind of obligation it is.
    pub kind: ObligationKind,
    /// The provision, or the act, that fixes the date.
    pub citation: String,
    /// One sentence saying what changes, for a person to read.
    pub summary: String,
    /// The product group it reaches, as its wire key. `None` for an act whose
    /// passport date this catalog records with no product group it models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_group: Option<String>,
    /// The product categories its source narrows it to, as wire names.
    ///
    /// Empty when the source does not narrow it, which is **not** a statement
    /// that every category is reached: an act's passport date is recorded per
    /// product group, so its event lists no categories even where the act's own
    /// scope is narrower. The battery passport, for one, reaches only EV, LMT
    /// and industrial batteries over 2 kWh.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    /// Whether [`Self::date`] traces to an adopted text.
    pub basis: DateBasis,
}
