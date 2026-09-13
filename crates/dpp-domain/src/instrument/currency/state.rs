//! [`CurrencyState`] — whether an act is still law, and which text to cite.

use serde::{Deserialize, Serialize};

/// Whether an act is still law, and which text carries it.
///
/// # Why this is not [`InstrumentStatus`](crate::instrument::InstrumentStatus)
///
/// That type is a **legislative-progress** axis: it answers *"does a text exist
/// that can be cited by CELEX?"*. This one answers *"is that text still the
/// law, and is it the version to cite?"*. The two are orthogonal in both
/// directions — a repealed act was still adopted, and an adopted act may have
/// been rewritten underneath its own CELEX number without anything in the file,
/// the filename or the number changing.
///
/// The second direction is the dangerous one, because the citation still
/// resolves. An act cited from a pre-amendment text really is law and its
/// article numbers really do exist; they simply no longer say what they said.
///
/// # Why the variants are named for the citer's question
///
/// [`Consolidated`](Self::Consolidated) deliberately does **not** distinguish an
/// amendment from a corrigendum, though the two are different legal events. It
/// is the state EUR-Lex expresses by generating a consolidated text, and the
/// consequence for anyone citing the act is identical either way: the published
/// text is no longer the one to quote.
///
/// The worked example is Regulation (EU) 2024/1781 itself. Its consolidation
/// carries the provenance line `02024R1781 -- EN -- 28.06.2024 -- 000.001` and
/// the heading *"Corrected by:"* — consolidation number `000`, meaning no
/// amendment has been folded in at all. It is a corrigendum, and it still moves
/// the text a reader must cite. A model that recorded only amendments would
/// call that act unchanged and send the reader to the superseded text.
///
/// # Not modelled: amended with no consolidation yet
///
/// EUR-Lex generates consolidated texts on a lag, so an act amended recently is
/// briefly changed with no consolidated CELEX to point at. No act in this
/// catalog is in that state, and a variant nothing produces is a distinction
/// every caller must handle and none can test. The state to add, if one ever
/// lands, is `Amended` with the amending act's own CELEX in place of a
/// consolidated one.
///
/// Serialised internally tagged on `state`:
/// `{"state":"inForce"}`,
/// `{"state":"consolidated","asOf":"02023R1542-20260813"}`,
/// `{"state":"repealed","by":"32026R0248","on":"2026-02-22"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// `rename_all` renames the variants; `rename_all_fields` renames the fields
// inside them, and only the second reaches `as_of`. Without it the wire form
// silently becomes `as_of` while every neighbouring key is camelCase.
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[non_exhaustive]
pub enum CurrencyState {
    /// In force, and no change has been recorded against it: the act as
    /// published in the Official Journal is the text to cite.
    InForce,
    /// In force, and changed since publication — amended, corrected, or both.
    /// A consolidated text exists and supersedes the published one as the text
    /// to cite.
    Consolidated {
        /// CELEX of the consolidated text, e.g. `"02023R1542-20260813"`.
        ///
        /// The identifier rather than a bare date, because it is what someone
        /// actually needs in order to fetch the right version. The date alone
        /// leaves them to reconstruct the `0`-prefixed form, and getting it
        /// wrong silently serves the original — EUR-Lex does not answer 404 for
        /// a consolidation that was never generated, it answers with the
        /// published text.
        ///
        /// This names the **current** consolidation, which is not necessarily
        /// one anybody here holds a copy of.
        as_of: String,
    },
    /// No longer law. Citable for history, never as an obligation.
    Repealed {
        /// CELEX of the act that repealed it, e.g. `"32026R0248"`.
        by: String,
        /// ISO-8601 date the repeal took effect.
        ///
        /// The date the act ceased to apply, which is routinely later than the
        /// repealing act's own publication — and is the date that decides
        /// whether a product placed on the market was governed by it.
        on: String,
    },
}

impl CurrencyState {
    /// Whether the act is still law.
    ///
    /// `true` for both [`InForce`](Self::InForce) and
    /// [`Consolidated`](Self::Consolidated) — a consolidated act is no less
    /// binding, it is merely quoted from a different text.
    #[must_use]
    pub fn is_law(&self) -> bool {
        !matches!(self, Self::Repealed { .. })
    }

    /// The CELEX to cite this act from, where it differs from the act's own.
    ///
    /// `None` for an act whose published text still stands, and `None` for a
    /// repealed one — a repealed act is cited from whatever text was current
    /// when it applied, which is a question about a date rather than about the
    /// act, so this type does not answer it.
    #[must_use]
    pub fn cite_instead(&self) -> Option<&str> {
        match self {
            Self::Consolidated { as_of } => Some(as_of),
            _ => None,
        }
    }
}
