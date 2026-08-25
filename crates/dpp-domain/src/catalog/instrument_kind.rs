//! [`InstrumentKind`] — *what kind* of legal act an instrument is.

use serde::{Deserialize, Serialize};

/// What kind of legal act an instrument is — not *which* act, which is
/// [`Instrument::id`](crate::catalog::Instrument::id).
///
/// # Why the two were separated
///
/// The predecessor of this type fused them: `Regime::Espr` (a framework
/// regulation that adopts delegated acts) and `Regime::BatteryRegulation` (a
/// self-contained instrument) were sibling variants of one enum. A horizontal
/// act adopted *under* ESPR then had nowhere to live — it is neither `Espr`
/// itself nor a separate regime — and an act that shares another system's
/// carrier had no representation at all.
///
/// Splitting the axis gives each of the three shapes a home, and lets an
/// instrument name its framework through
/// [`Instrument::parent`](crate::catalog::Instrument::parent) rather than by
/// being collapsed into it.
///
/// Serialised as a bare string with [`InstrumentKind::Other`] absorbing any
/// value this build does not model — the same treatment its predecessor used,
/// for the same reason: a manifest naming a new kind must load rather than
/// break catalog loading outright.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
#[non_exhaustive]
pub enum InstrumentKind {
    /// A framework regulation that sets essential requirements and adopts
    /// delegated or implementing acts to apply them — ESPR (EU) 2024/1781, and
    /// the Ecodesign Directive 2009/125/EC before it. Its own articles bind; the
    /// product-level detail arrives in its children.
    Framework,
    /// An act adopted **under** a framework that carries the product-level
    /// requirements — an ESPR delegated act, whether it covers one product group
    /// or many horizontally. Names its framework in
    /// [`Instrument::parent`](crate::catalog::Instrument::parent).
    ///
    /// This variant was missing from the first sketch of this axis, which had
    /// only framework / direct / adjacent. That omission would have forced every
    /// ESPR product-group obligation — the single most important instrument shape
    /// this catalog has to carry, since that is how all of them will arrive — to
    /// be recorded as either the framework itself or a standalone act, and
    /// neither is true.
    Delegated,
    /// A self-contained instrument carrying its own passport obligation, owing
    /// nothing to a framework — the Batteries, Toy Safety, Detergents,
    /// Construction Products and End-of-Life Vehicles Regulations.
    Direct,
    /// An act that imposes product data duties discharged through *another*
    /// system, so it shares or displaces a carrier rather than creating a
    /// passport. Regs (EU) 2023/1670 and 2023/1669 route through EPREL; PPWR
    /// points at the packaged product's existing passport.
    ///
    /// This is the kind the previous model could not express, and where the
    /// `electronics` defect lived: an adjacent act was recorded as though it
    /// created a passport obligation of its own.
    Adjacent,
    /// An act adopted **under** a framework that fixes the *procedure or format*
    /// by which an obligation is met, rather than the obligation itself — an EU
    /// implementing act. Names its framework in
    /// [`Instrument::parent`](crate::catalog::Instrument::parent).
    ///
    /// Distinct from [`Self::Delegated`] because the Treaty distinction is real
    /// and the two do different work: a delegated act may supplement or amend
    /// non-essential elements of the basic act, while an implementing act only
    /// lays down uniform conditions for implementing it. Impl. Reg. (EU) 2026/2
    /// is the clearest case in this catalog — it creates no duty at all, it
    /// prescribes the format of a disclosure ESPR Art. 24 already required.
    ///
    /// Recorded as its own kind for the same reason `Delegated` was: forcing an
    /// implementing act into `Delegated` would assert it can do something it
    /// cannot, and forcing it into `Other` would drop a distinction the law
    /// draws.
    Implementing,
    /// A kind this build does not model, holding its manifest spelling verbatim.
    Other(String),
}

impl InstrumentKind {
    /// Manifest spelling of a modelled variant. `None` for
    /// [`InstrumentKind::Other`], which carries its own string.
    #[must_use]
    pub fn wire_str(&self) -> Option<&'static str> {
        Some(match self {
            Self::Framework => "framework",
            Self::Delegated => "delegated",
            Self::Direct => "direct",
            Self::Adjacent => "adjacent",
            Self::Implementing => "implementing",
            Self::Other(_) => return None,
        })
    }
}

impl From<String> for InstrumentKind {
    fn from(s: String) -> Self {
        match s.as_str() {
            "framework" => Self::Framework,
            "delegated" => Self::Delegated,
            "direct" => Self::Direct,
            "adjacent" => Self::Adjacent,
            "implementing" => Self::Implementing,
            _ => Self::Other(s),
        }
    }
}

impl From<InstrumentKind> for String {
    fn from(kind: InstrumentKind) -> Self {
        match kind {
            InstrumentKind::Other(s) => s,
            modelled => modelled
                .wire_str()
                .expect("every non-Other variant has a wire string")
                .to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_modelled_variant_round_trips_as_a_bare_string() {
        for kind in [
            InstrumentKind::Framework,
            InstrumentKind::Delegated,
            InstrumentKind::Direct,
            InstrumentKind::Adjacent,
            InstrumentKind::Implementing,
        ] {
            let json = serde_json::to_string(&kind).expect("serialise");
            assert!(
                json.starts_with('"'),
                "{kind:?} must render as a string, got {json}"
            );
            assert_eq!(
                serde_json::from_str::<InstrumentKind>(&json).expect("deserialise"),
                kind
            );
        }
    }

    #[test]
    fn an_unmodelled_kind_is_absorbed_not_rejected() {
        let parsed: InstrumentKind =
            serde_json::from_str("\"international-agreement\"").expect("must not fail");
        assert_eq!(
            parsed,
            InstrumentKind::Other("international-agreement".to_owned())
        );
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            "\"international-agreement\""
        );
    }
}
