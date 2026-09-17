//! [`ProductIdentifier`] — a unique product identifier under EN 18219 clause 5.

use serde::{Deserialize, Serialize};

use super::super::gtin::Gtin;
use super::error::ProductIdentifierError;

/// The DID methods EN 18219 scheme 3 names.
///
/// The standard describes scheme 3 as Decentralized Identifiers and names these
/// three as the admissible methods, `did:web` being the lightweight non-DLT
/// option. Closed rather than open because an identifier exists to be followed:
/// a method no reader can resolve identifies nothing, and accepting one would
/// let a passport be created that is unreachable by design.
const DID_METHODS: [&str; 3] = ["web", "ethr", "ebsi"];

/// A unique product identifier, in whichever EN 18219 clause 5 scheme issued it.
///
/// ✅ COMPLIANCE-PIN: EN 18219:2026 clause 5.1 — a unique product identifier
/// satisfies the general principles of clause 4 **and** complies with **one of**
/// the ID schemes in clause 5. The schemes are alternatives, not a hierarchy.
/// EN 18219 is one of the six standards cited by Commission Implementing
/// Decision (EU) 2026/1736, and a presumption route under ESPR **Art. 41(2)**.
///
/// # Why this is an enum and not an optional GTIN
///
/// Every product-group payload once declared `gtin: Gtin` — not `Option<Gtin>` —
/// so a GTIN was structurally required to create a passport at all. A GTIN needs
/// a GS1 Company Identification Number, which is a recurring paid subscription,
/// and clause 5 admits four other schemes of which **two are self-issuing**. So
/// the type foreclosed the schemes with no external dependency and kept the only
/// one with an annual bill attached — a commercial position expressed as a
/// compile error, which is not where such a position should live.
///
/// The standard's own informative Annex B, Table B.4, makes the comparison
/// plainly: every scheme needs a registered web domain; scheme 1 *additionally*
/// needs the CIN. Scheme 3's prerequisites are a strict subset of scheme 1's,
/// and the same table rates its sovereignty over the identifier's data highest
/// of the three — full control, verifiable cryptographically.
///
/// # What each variant does and does not check
///
/// This is the vocabulary tier: a type here names a thing and decides nothing
/// about who may use it. It still validates what it can *read*, the way [`Gtin`]
/// verifies a check digit — an identifier that cannot be parsed is not a
/// weaker identifier, it is a typo nobody caught.
///
/// 🚨 **Scheme 2 is deliberately under-validated, and that is recorded rather
/// than hidden.** Its format is EN IEC 61406-1/-2, which this project does not
/// hold. Checking only that the value is an absolute `http(s)` URL is what can
/// be checked against a text in hand; asserting 61406 conformance would be a
/// claim made from a standard nobody here has read. See
/// [`Self::IdentificationLink`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scheme", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ProductIdentifier {
    /// **Scheme 1** — a web-enabled structured path/query identifier, GS1
    /// branch: a GS1 Digital Link keyed on a GTIN.
    ///
    /// Clause 5 admits the ASC MH10.8.2 Data Identifier branch of this scheme
    /// too. It is not modelled: nothing in this workspace issues one, and an
    /// unused variant would be a guess at a shape rather than a reading of one.
    #[serde(rename_all = "camelCase")]
    Gs1 {
        /// The trade item number, check digit verified.
        gtin: Gtin,
    },
    /// **Scheme 2** — an Identification Link per EN IEC 61406-1/-2.
    ///
    /// Self-issuing: the operator needs its own web domain and nothing else.
    ///
    /// 🚨 Only checked to be an absolute `http`/`https` URL. EN IEC 61406 is not
    /// held by this project, so its format rules are unverified here, and a
    /// value that passes this check is **not** thereby conformant. Recorded on
    /// the variant so a reader does not infer a conformance claim from the
    /// absence of an error.
    #[serde(rename_all = "camelCase")]
    IdentificationLink {
        /// The identification link URL.
        url: String,
    },
    /// **Scheme 3** — a Decentralized Identifier (W3C DID v1.0:2022).
    ///
    /// Self-issuing, and the closest to what this workspace already builds:
    /// `dpp-vc` produces `did:web` documents today.
    #[serde(rename_all = "camelCase")]
    Did {
        /// The DID, e.g. `did:web:passports.example.com:item:0001`.
        did: String,
    },
}

impl ProductIdentifier {
    /// A scheme 1 identifier from an already-validated GTIN.
    #[must_use]
    pub fn gs1(gtin: Gtin) -> Self {
        Self::Gs1 { gtin }
    }

    /// Parse a scheme 2 identification link.
    ///
    /// # Errors
    ///
    /// [`ProductIdentifierError::NotAWebUrl`] if `url` is not an absolute
    /// `http`/`https` URL. See the variant's note on what is *not* checked.
    pub fn identification_link(url: &str) -> Result<Self, ProductIdentifierError> {
        let rest = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"));
        match rest {
            Some(host) if !host.is_empty() => Ok(Self::IdentificationLink {
                url: url.to_owned(),
            }),
            _ => Err(ProductIdentifierError::NotAWebUrl(url.to_owned())),
        }
    }

    /// Parse a scheme 3 DID.
    ///
    /// # Errors
    ///
    /// [`ProductIdentifierError::NotADid`] if the value is not
    /// `did:<method>:<id>`, [`ProductIdentifierError::UnsupportedDidMethod`] if
    /// the method is not one clause 5 names, and
    /// [`ProductIdentifierError::EmptyDidMethodId`] if nothing follows it.
    pub fn did(did: &str) -> Result<Self, ProductIdentifierError> {
        let rest = did
            .strip_prefix("did:")
            .ok_or_else(|| ProductIdentifierError::NotADid(did.to_owned()))?;
        let (method, method_id) = rest
            .split_once(':')
            .ok_or_else(|| ProductIdentifierError::NotADid(did.to_owned()))?;
        if !DID_METHODS.contains(&method) {
            return Err(ProductIdentifierError::UnsupportedDidMethod(
                method.to_owned(),
            ));
        }
        if method_id.is_empty() {
            return Err(ProductIdentifierError::EmptyDidMethodId(did.to_owned()));
        }
        Ok(Self::Did {
            did: did.to_owned(),
        })
    }

    /// The GTIN, when this identifier was issued under scheme 1.
    ///
    /// `None` for schemes 2 and 3, which have no GTIN and are not lesser for
    /// it — a caller reaching for one needs a path that does not require it.
    #[must_use]
    pub fn gtin(&self) -> Option<&Gtin> {
        match self {
            Self::Gs1 { gtin } => Some(gtin),
            _ => None,
        }
    }

    /// The identifier as a single string, whichever scheme issued it.
    ///
    /// For scheme 1 this is the 14-digit GTIN, not a Digital Link URL: building
    /// the URL needs a resolver host, which is a deployment fact and not
    /// something this tier knows.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Gs1 { gtin } => gtin.as_str(),
            Self::IdentificationLink { url } => url,
            Self::Did { did } => did,
        }
    }
}
