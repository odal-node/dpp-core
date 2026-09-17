//! [`RegistryBasis`] — the tier itself.

use serde::Serialize;

/// What this crate knows a wire detail from.
///
/// 🚨 **Describes this crate's evidence, never the registry's requirements.**
/// [`Observed`](Self::Observed) means someone read a value off a real artefact
/// on a real date. It does not mean the registry has specified anything, and
/// nothing here should be quoted as *the registry requires*.
///
/// Serialisable so a consumer can publish its own provenance; deliberately not
/// deserialisable, because a basis read off the wire would be a claim about us
/// made by someone else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "basis")]
#[non_exhaustive]
pub enum RegistryBasis {
    /// 👁️ Read off an artefact the registry itself publishes — today, its
    /// unauthenticated web client — on the given date.
    ///
    /// One consumer's behaviour on one day. Better evidence than an invention
    /// and still not a specification.
    #[serde(rename_all = "camelCase")]
    Observed {
        /// ISO-8601 date the value was read or last re-confirmed.
        ///
        /// A `&'static str` rather than a date type: this is a provenance note
        /// compiled into the binary, and nothing computes with it. Parsing it
        /// would invite arithmetic that implies a precision the observation
        /// does not have.
        on: &'static str,
    },
    /// ⚠️ Nothing behind it. A shape the flow needs in order to be expressible,
    /// which the registry has not published.
    ///
    /// Kept rather than omitted, because a caller has to be able to *name* a
    /// route; the marker is what stops naming it turning into knowing it.
    Assumed,
}

impl RegistryBasis {
    /// Whether anything outside this crate backs the value.
    ///
    /// The question a conformance-claiming test asks, and the reason this type
    /// exists: `false` means the value is ours, and a run that depended on it
    /// demonstrated the crate agreeing with itself.
    #[must_use]
    pub fn is_observed(self) -> bool {
        matches!(self, Self::Observed { .. })
    }

    /// The observation date, or `None` when nothing was observed.
    #[must_use]
    pub fn observed_on(self) -> Option<&'static str> {
        match self {
            Self::Observed { on } => Some(on),
            Self::Assumed => None,
        }
    }
}
