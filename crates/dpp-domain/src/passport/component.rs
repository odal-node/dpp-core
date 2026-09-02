//! [`ComponentRef`] — a qualified downward edge to one constituent passport.

use serde::{Deserialize, Serialize};

use super::reference::PassportRef;

/// How much of a constituent a bill of materials entry covers.
///
/// Product-group-neutral by construction. Core carries the number and the unit
/// and interprets neither: a `unit` is an opaque label here, not a member of a
/// vocabulary this crate validates against. Deciding that "cell" is counted and
/// "aluminium" is weighed is a product-group judgement, and no delegated act
/// defines it for any product group in force today.
///
/// `unit: None` means a dimensionless count — two of a thing, rather than two
/// kilograms of it.
///
/// `f64` matches the convention every other physical quantity in this crate
/// already uses (see `MaterialEntry::weight_kg` and `recycled_pct`). It is the
/// wrong type for exact arithmetic, and core performs none on it; a consumer
/// that needs exactness should not be reading it back out of a passport to do
/// arithmetic on anyway.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quantity {
    /// The magnitude.
    pub value: f64,
    /// The unit `value` is expressed in, as the declaring operator wrote it.
    /// `None` means a dimensionless count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// One constituent in a product's bill of materials.
///
/// Wraps [`PassportRef`] with the two qualifiers a downward edge needs. The
/// reference alone records *where* a constituent's passport is and *which hash*
/// pins it, but never how much of it or what part it plays — and a bill of
/// materials that cannot answer "how much of what, where" does not answer the
/// question a bill of materials exists for.
///
/// **Both qualifiers are optional, and core never interprets either.** That is
/// deliberate rather than unfinished: a battery module, a fibre lot and an
/// electronics sub-assembly are not the same kind of thing, and no delegated act
/// defines "component" or its granularity for any of our in-force product
/// groups. Core's job is to carry a pinned reference plus product-group-neutral
/// qualifiers and let product-group plugins interpret them. Hard-coding a
/// granularity here would commit every product group to one product group's
/// answer.
///
/// # This edge is a claim, not a consented fact
///
/// The hash-pin proves the target has not been modified. It does not prove the
/// target's operator agreed to being listed as a constituent, and BOM edges
/// deliberately carry **no** consent requirement: demanding a supplier signature
/// for every assembly is not something any real supply chain produces. A
/// `componentRef` is therefore a *claim by the assembler*, pinned so it cannot
/// be tampered with, and the verification walk reports it as exactly that.
///
/// This is the asymmetry with the upward direction, where
/// [`DerivationRef`](super::DerivationRef) moves regulatory responsibility under
/// Reg. (EU) 2023/1542 Art. 77(7) and so does need a consent artefact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentRef {
    /// Where to fetch the constituent's passport, and the hash pinning its
    /// signed public view.
    pub reference: PassportRef,
    /// How much of this constituent the assembly contains. `None` where the
    /// declaring operator did not state one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Quantity>,
    /// What part this constituent plays, in whatever terms the product group
    /// uses — "cell", "outer shell", "warp yarn". A free string because the
    /// vocabulary is the product group's to define, not core's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}
