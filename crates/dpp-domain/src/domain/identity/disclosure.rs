//! [`Disclosure`] — how restricted a field is, and the key a disclosure set gets.

use serde::{Deserialize, Serialize};

/// How restricted a field is — the counterpart to [`Audience`].
///
/// Named for the Annex XIII point each class corresponds to, and kept
/// product group-agnostic so non-battery product groups reuse the same vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Disclosure {
    /// Publicly accessible. Annex XIII point 1.
    Public,
    /// Detailed composition, dismantling information, safety measures.
    /// Annex XIII point 2 — visible to **both** non-public audiences.
    Restricted,
    /// Conformity evidence: results of test reports. Annex XIII point 3 —
    /// authorities only.
    Conformity,
    /// Information and data relating to an **individual** item: use history,
    /// cycle counts, negative events, state of health, status. Annex XIII
    /// point 4 — legitimate interest only, and explicitly **not** authorities.
    Individual,
}

/// Disclosure class of every top-level passport field that is not public.
///
/// **The single source for this fact.** `Passport::redact` and the crypto
/// layer's `ProductGroupAccessPolicy::passport_default()` both read it, because they
/// previously each carried their own copy and drifted: the policy classified
/// `lintResult` as restricted while `redact` never removed it, so a public view
/// built through the domain path disclosed it.
///
/// Fields absent from this list are [`Disclosure::Public`].
pub const PASSPORT_FIELD_DISCLOSURE: &[(&str, Disclosure)] = &[
    ("batchId", Disclosure::Restricted),
    // Advisory plausibility output, re-computable after publish and carrying
    // free-text findings about our own data quality — operator- and
    // auditor-facing, not consumer-facing.
    ("lintResult", Disclosure::Restricted),
    ("jwsSignature", Disclosure::Conformity),
    ("retentionLocked", Disclosure::Conformity),
];

impl Disclosure {
    /// How many of the three audiences may see this class.
    ///
    /// Not an ordering of the lattice — there isn't one. It is the only totally
    /// ordered thing the lattice offers, which is what a deterministic tie-break
    /// needs.
    const fn audience_count(self) -> u8 {
        match self {
            // Everyone.
            Self::Public => 3,
            // Legitimate interest and authorities, not the public.
            Self::Restricted => 2,
            // Exactly one audience each, and not the same one.
            Self::Conformity | Self::Individual => 1,
        }
    }

    /// The more restrictive of two classes — the one fewer audiences may see.
    ///
    /// Exists so that an ambiguous lookup resolves the safe way and resolves it
    /// **identically every time**. Used by
    /// [`ProductGroupAccessPolicy::disclosure_for_field`](crate::access::ProductGroupAccessPolicy::disclosure_for_field)
    /// when two normalized-equal keys both match.
    ///
    /// `Conformity` and `Individual` are genuinely incomparable — Art. 77(2)
    /// gives each to one audience, and neither audience contains the other, so
    /// no class means "withheld from both". The tie-break returns `Individual`,
    /// which is a choice rather than a derivation, and it is only ever reached
    /// by a policy that declares one field name in both classes. That is an
    /// authoring error a schema cannot commit: `access::tests` rejects it at
    /// build time.
    #[must_use]
    pub const fn most_restrictive(self, other: Self) -> Self {
        if other.audience_count() <= self.audience_count() {
            other
        } else {
            self
        }
    }

    /// The stable wire token for this class, used to build a disclosure-set key.
    ///
    /// Deliberately not `Serialize`-derived: this string is baked into stored
    /// artefact keys, so it must be stable independently of any future serde
    /// attribute change on the enum.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Restricted => "restricted",
            Self::Conformity => "conformity",
            Self::Individual => "individual",
        }
    }
}

/// Every disclosure class, in the fixed order a [`disclosure_key`] uses.
///
/// Ordering is by Annex XIII point number, and it is part of the key format:
/// two nodes must produce byte-identical keys for the same set.
pub(super) const DISCLOSURE_ORDER: &[Disclosure] = &[
    Disclosure::Public,
    Disclosure::Restricted,
    Disclosure::Conformity,
    Disclosure::Individual,
];

/// Name a set of disclosure classes: the classes' tokens in Annex XIII order,
/// joined with `+` — e.g. `public+restricted+individual`.
///
/// **This is how durable artefacts are keyed, and it must never be an audience
/// name.** ESPR uses a ~14-class actor vocabulary that is not battery's
/// three-audience lattice, and the delegated act mapping actors to data does not
/// exist yet. A signature or audit row keyed `"legitimateInterest"` would have
/// to be migrated the day that mapping lands; one keyed by the disclosure set it
/// actually covers keeps meaning exactly what it always meant, and a new actor
/// taxonomy becomes a new mapping onto the same keys.
#[must_use]
pub fn disclosure_key(classes: &[Disclosure]) -> String {
    DISCLOSURE_ORDER
        .iter()
        .filter(|c| classes.contains(c))
        .map(|c| c.token())
        .collect::<Vec<_>>()
        .join("+")
}
