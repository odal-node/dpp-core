//! [`TrustServiceStatus`] — what a Member State says about one trust service.

use serde::{Deserialize, Serialize};

/// The status a supervisory body has recorded for a listed trust service.
///
/// ETSI TS 119 612 clause 5.5.4 and Annex D.5. The values are URIs under
/// `http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/`.
///
/// # Why only two are named
///
/// The standard defines thirteen status URIs, and it would be easy to model all
/// thirteen as equals. They are not equals. Clause 5.5.4 says that for a service
/// of a type listed in clause 5.5.1.1 — which is every service type that confers
/// qualified status, including the ones this crate cares about — the status
/// **shall be either `granted` or `withdrawn`**. Nothing else is permitted
/// there.
///
/// The remaining eleven belong to other axes entirely: `recognisedatnationallevel`
/// and `deprecatedatnationallevel` are for nationally-defined services
/// (clauses 5.5.1.2 and 5.5.1.3), `setbynationallaw` is for national root CAs,
/// and `accredited` / `undersupervision` / `supervisionceased` and their
/// relatives are pre-eIDAS survivals from the Directive 1999/93/EC regime.
///
/// Folding those into the same enum as `granted` invites the reading that
/// `undersupervision` is a lesser kind of qualified, or that
/// `recognisedatnationallevel` is a weaker `granted`. Neither is true: they
/// answer a different question, asked of a different kind of service.
/// [`Self::Other`] keeps them readable without making them comparable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TrustServiceStatus {
    /// `granted` — the supervisory body **has granted qualified status** to this
    /// service, for this provider.
    ///
    /// Annex D.5: granted "following ex ante and active approval activities", by
    /// the supervisory body on behalf of the Member State, both to the service
    /// and to the trust service provider that offers it.
    ///
    /// **This is the only status that means qualified.**
    Granted,
    /// `withdrawn` — qualified status was **withdrawn, or never granted**.
    ///
    /// Annex D.5 covers both cases in one value, which is worth noticing: a
    /// service that was refused at the outset and one that lost its status after
    /// years of operation are indistinguishable by status alone. Only the
    /// service history separates them — see
    /// [`TrustServiceHistory`](crate::trusted_list::TrustServiceHistory).
    Withdrawn,
    /// Any other status URI, kept verbatim.
    ///
    /// Not an error and not a lesser `Granted`. A national-level or pre-eIDAS
    /// status, which says nothing either way about qualified status under
    /// Regulation (EU) No 910/2014.
    Other(String),
}

/// `http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted`
pub const GRANTED_URI: &str = "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted";
/// `http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn`
pub const WITHDRAWN_URI: &str = "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn";

impl TrustServiceStatus {
    /// Read a status from its TS 119 612 URI.
    ///
    /// Never fails: an unrecognised URI becomes [`Self::Other`] rather than an
    /// error, because a trusted list that adds a status this build has not seen
    /// is not malformed — it is newer. Refusing it would turn a Member State's
    /// routine publication into an outage, and the honest answer to "is this
    /// granted?" for an unknown status is "no", which [`Self::is_granted`]
    /// already gives.
    #[must_use]
    pub fn from_uri(uri: &str) -> Self {
        match uri {
            GRANTED_URI => Self::Granted,
            WITHDRAWN_URI => Self::Withdrawn,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The URI this status is written as.
    #[must_use]
    pub fn as_uri(&self) -> &str {
        match self {
            Self::Granted => GRANTED_URI,
            Self::Withdrawn => WITHDRAWN_URI,
            Self::Other(uri) => uri,
        }
    }

    /// Whether this status confers qualified status.
    ///
    /// True for [`Self::Granted`] and nothing else, deliberately. The method
    /// exists so that no call site has to decide whether some other status is
    /// "close enough" — the answer is always no, and stating it once is what
    /// keeps it that way.
    #[must_use]
    pub fn is_granted(&self) -> bool {
        matches!(self, Self::Granted)
    }
}
