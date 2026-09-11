//! [`TrustServiceType`] — which kind of trust service a trusted-list entry is.

use serde::{Deserialize, Serialize};

/// A trust service type, as a TS 119 612 service type URI.
///
/// A newtype over the URI rather than an enum, because the set is **open**: ETSI
/// adds service types as the Regulation grows new trust services, and eIDAS 2
/// did exactly that. An enum would turn every such addition into a build break
/// for consumers, and — worse — would tempt a `_` arm that silently classifies
/// an unknown service type as something it is not.
///
/// The types this crate can name a use for are constants below. Anything else
/// round-trips as itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrustServiceType(String);

impl TrustServiceType {
    /// `…/Svctype/CA/QC` — a CA issuing **qualified certificates**.
    ///
    /// The service type behind Art. 32(1)(a)–(b) as applied to seals by Art. 40:
    /// that the certificate was a qualified certificate *issued by a qualified
    /// trust service provider*. A seal's certificate chains to a CA, and this is
    /// the entry that says whether that CA was granted qualified status.
    ///
    /// Note it does **not** distinguish seals from signatures — one CA/QC entry
    /// may cover both. What the certificate is *for* is read from the
    /// certificate's own QCStatements, not from the service type.
    pub const QUALIFIED_CERTIFICATE_CA: &'static str = "http://uri.etsi.org/TrstSvc/Svctype/CA/QC";

    /// `…/Svctype/RemoteQSealCDManagement/Q` — **management of remote qualified
    /// electronic seal creation devices**.
    ///
    /// The Art. 39a service, and the one a cloud-sealing arrangement turns on.
    /// eIDAS 2 made this management its own qualified trust service, and the
    /// transitional in Art. 51(3) that allowed it to be performed without
    /// qualified status **expired on 21 May 2026**.
    ///
    /// So a provider that holds the certificate leg and not this one cannot
    /// supply the creation-device limb of Art. 3(27), and the seal it produces is
    /// not qualified however good the certificate is. This constant is the exact
    /// thing to look for in a trusted list when asking a provider to prove
    /// otherwise.
    pub const REMOTE_QSEAL_CD_MANAGEMENT: &'static str =
        "http://uri.etsi.org/TrstSvc/Svctype/RemoteQSealCDManagement/Q";

    /// `…/Svctype/RemoteSealCDManagement` — the **non-qualified** counterpart.
    ///
    /// Present so the two can be told apart deliberately. A provider listed
    /// under this type manages remote seal creation devices that are **not**
    /// qualified, which is precisely the arrangement that does not satisfy
    /// Art. 3(27). The names differ by four characters and the legal effect
    /// differs completely.
    pub const REMOTE_SEAL_CD_MANAGEMENT: &'static str =
        "http://uri.etsi.org/TrstSvc/Svctype/RemoteSealCDManagement";

    /// `…/Svctype/TSA/QTST` — a **qualified** timestamping authority.
    ///
    /// What a `B-T` seal's timestamp token has to come from for that timestamp to
    /// carry the Art. 41(2) presumption of accuracy.
    pub const QUALIFIED_TIMESTAMP_AUTHORITY: &'static str =
        "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST";

    /// `…/Svctype/QESValidation/Q` — a **qualified validation service** for
    /// qualified signatures and seals (Art. 33, applied to seals by Art. 40).
    pub const QUALIFIED_VALIDATION_SERVICE: &'static str =
        "http://uri.etsi.org/TrstSvc/Svctype/QESValidation/Q";

    /// `…/Svctype/PSES/Q` — a **qualified preservation service** for qualified
    /// signatures and seals (Art. 34, applied to seals by Art. 40).
    pub const QUALIFIED_PRESERVATION_SERVICE: &'static str =
        "http://uri.etsi.org/TrstSvc/Svctype/PSES/Q";

    /// Wrap a service type URI.
    #[must_use]
    pub fn new(uri: impl Into<String>) -> Self {
        Self(uri.into())
    }

    /// The URI.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is one of the service types that confers **qualified** status.
    ///
    /// Read off the URI shape that TS 119 612 uses for it: the qualified
    /// variants are the ones ending `/Q`, `/QC` or `/QTST`. This is a *syntactic*
    /// test on a naming convention, and it is offered as a convenience for
    /// reading a list, never as the basis of a compliance decision — the status
    /// is what decides that, and a service of a qualified type can perfectly well
    /// be `withdrawn`.
    #[must_use]
    pub fn looks_qualified(&self) -> bool {
        self.0.ends_with("/Q") || self.0.ends_with("/QC") || self.0.ends_with("/QTST")
    }
}

impl std::fmt::Display for TrustServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
