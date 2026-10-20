//! [`ProductIdentifier`] — a unique product identifier under EN 18219 clause 5.

use dpp_rules::common::identifier::{DidRejection, WebUrlRejection, check_did, check_web_url};
use serde::{Deserialize, Serialize};

use super::super::gtin::Gtin;
use super::error::ProductIdentifierError;

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
/// a GS1 Company Identification Number, which only GS1 issues, and clause 5
/// admits four other schemes of which **two are self-issuing**. So the type
/// foreclosed the schemes that depend on no outside issuer and kept the one that
/// does — a choice of issuer expressed as a compile error, which is not where
/// such a choice should live.
///
/// The standard's own informative Annex B, Table B.4, sets the prerequisites
/// side by side: every scheme needs a registered web domain; scheme 1
/// *additionally* needs the CIN. Scheme 3's prerequisites are a strict subset of
/// scheme 1's, and the same table rates its sovereignty over the identifier's
/// data highest of the three — full control, verifiable cryptographically.
///
/// # What each variant does and does not check
///
/// This is the vocabulary tier: a type here names a thing and decides nothing
/// about who may use it. It still validates what it can *read*, the way [`Gtin`]
/// verifies a check digit — an identifier that cannot be parsed is not a
/// weaker identifier, it is a typo nobody caught.
///
/// 🚨 **Scheme 2 is deliberately under-validated, and that is recorded rather
/// than hidden.** Its format is specified by EN IEC 61406-1/-2. This type checks
/// only that the value is an absolute `http(s)` URL, which is a shape check and
/// not a conformance verdict — a caller needing conformance to that standard
/// must establish it elsewhere. See [`Self::IdentificationLink`].
///
/// Whatever each variant does check, it checks on the way in **and** on the way
/// back: deserialisation is routed through the constructors, so a stored
/// identifier that could not have been built cannot be read either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scheme", rename_all = "camelCase")]
#[serde(try_from = "Wire")]
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
    /// 🚨 Only checked to be an absolute `http`/`https` URL written in printable
    /// ASCII, which EN 18219:2026 clause 4.3.2 requires of every identifier: a
    /// non-ASCII host is refused and written in punycode instead, any other
    /// character outside printable ASCII is percent-encoded, and neither is done
    /// for the caller. EN IEC 61406's own format rules are not applied here, so
    /// a value that passes this check is **not** thereby conformant. Recorded on
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
    /// `http`/`https` URL, [`ProductIdentifierError::NonAsciiHost`] if its host
    /// is not in ASCII (punycode) form, and
    /// [`ProductIdentifierError::NotPrintableAscii`] if anything else in it is
    /// outside printable ASCII, which is percent-encoded. A link is refused
    /// rather than converted, because the identifier stored has to be the one
    /// that is printed. See the variant's note on what is *not* checked.
    ///
    /// This is the rule for **issuing** an identifier. A document being *read*
    /// is held to it for everything but printable ASCII: a link that was issued
    /// before the rule existed, or by an operator that has not upgraded, is
    /// signed and cannot be rewritten, so it is read as it is. The reason is on
    /// the type that does the reading.
    pub fn identification_link(url: &str) -> Result<Self, ProductIdentifierError> {
        // The syntax lives in `dpp_rules` so the plugin SDK checks the same
        // thing this does — it previously had a weaker copy of its own. Each
        // rejection keeps its own error here, because the remedies differ.
        check_web_url(url).map_err(|rejection| match rejection {
            WebUrlRejection::NotAnAbsoluteWebUrl => {
                ProductIdentifierError::NotAWebUrl(url.to_owned())
            }
            WebUrlRejection::NonAsciiHost(character) => ProductIdentifierError::NonAsciiHost {
                url: url.to_owned(),
                character,
            },
            WebUrlRejection::NotPrintableAscii(character) => {
                ProductIdentifierError::NotPrintableAscii {
                    url: url.to_owned(),
                    character,
                }
            }
        })?;
        Ok(Self::IdentificationLink {
            url: url.to_owned(),
        })
    }

    /// Parse a scheme 3 DID.
    ///
    /// # Errors
    ///
    /// [`ProductIdentifierError::NotADid`] if the value does not satisfy the W3C
    /// DID v1.0 clause 3.1 grammar, [`ProductIdentifierError::UnsupportedDidMethod`]
    /// if the method is not one clause 5 names, and
    /// [`ProductIdentifierError::EmptyDidMethodId`] if nothing follows it.
    pub fn did(did: &str) -> Result<Self, ProductIdentifierError> {
        // The grammar lives in `dpp_rules` so the plugin SDK checks the same
        // thing this does. Each rejection keeps its own error here: which of
        // the three ways a DID is wrong is worth telling the caller.
        check_did(did).map_err(|rejection| match rejection {
            DidRejection::UnsupportedMethod(method) => {
                ProductIdentifierError::UnsupportedDidMethod(method.to_owned())
            }
            DidRejection::EmptyMethodId => ProductIdentifierError::EmptyDidMethodId(did.to_owned()),
            DidRejection::Malformed => ProductIdentifierError::NotADid(did.to_owned()),
        })?;
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

    /// What **kind of value** this identifier carries — `"gtin"`,
    /// `"identificationLink"` or `"did"`.
    ///
    /// 🚨 **Not the same question as the clause 5 scheme**, and the two answers
    /// differ for scheme 1. The serde tag is `"gs1"`, which names the *scheme*
    /// that issued the identifier; this names the *thing the value is*. They
    /// coincide for schemes 2 and 3 and not for scheme 1, which is exactly
    /// where a caller conflating them would go wrong without failing.
    ///
    /// # Why this lives here rather than in each consumer
    ///
    /// Every surface that labels an identifier needs it — the registry wire
    /// `scheme`, an AAS `specificAssetId` name — and none of them can compute
    /// it safely on its own: this enum is `#[non_exhaustive]`, so a match from
    /// another crate must carry a wildcard arm, and a wildcard that produced a
    /// label would name a scheme nobody has mapped as though it had been.
    /// Authored here, the match is exhaustive and the function is total.
    #[must_use]
    pub fn value_kind(&self) -> &'static str {
        match self {
            Self::Gs1 { .. } => "gtin",
            Self::IdentificationLink { .. } => "identificationLink",
            Self::Did { .. } => "did",
        }
    }

    /// Whether [`as_str`](Self::as_str) is already a resolvable URI.
    ///
    /// 🚨 Schemes 2 and 3 are URIs in their own right — an `http(s)` URL and a
    /// DID. Scheme 1 is a bare 14-digit number and is not. A caller that needs
    /// an identifier-shaped URI must wrap the third case and must **not** wrap
    /// the other two: nesting a URI inside a URN produces
    /// `urn:…:did:web:example.com`, which is a URN whose namespace-specific
    /// string is another scheme, and no resolver treats that as either.
    #[must_use]
    pub const fn is_uri(&self) -> bool {
        matches!(self, Self::IdentificationLink { .. } | Self::Did { .. })
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

/// The wire shape a stored identifier is read into, before validation.
///
/// 🚨 Deriving `Deserialize` on [`ProductIdentifier`] itself built the two
/// self-issuing arms field-by-field and never called their constructors, so a
/// document holding an unresolvable DID or a hostless URL read back happily —
/// exactly the values the constructors exist to refuse. `Gtin` does not have
/// that problem, because it validates inside its own `Deserialize`; that is what
/// made the gap easy to miss, since scheme 1 was safe and the other two were
/// not. Reading goes through here so a document is held to the same rules as a
/// constructor argument, **with one deliberate exception**, below.
///
/// 🚨 **Reading is not issuing, for printable ASCII.** A link outside printable
/// ASCII is refused when one is issued ([`ProductIdentifier::identification_link`])
/// and when a plugin validates one, and it is **read** all the same. The product
/// identifier is in the signed public view, so it is on the wire between
/// operators as well as on disk, and a signed passport fetched from another
/// operator cannot be rewritten by anyone: a reader that refused it would refuse
/// correct, signed data permanently, and report a node that has not upgraded as
/// having tampered with it. The fetched side is the one that decides, because it
/// is the stricter. A stored document is not turned away either, which would be
/// an unannounced non-additive change to what is on disk. What stops a
/// non-conformant link persisting is the write path: the plugin refuses it the
/// next time the passport is written, which names the field and the remedy, and
/// the node that owns the bytes can fix it. A link that is not a web URL at all
/// is still refused on read, as it always was.
///
/// The variants and their serde attributes must mirror [`ProductIdentifier`]'s
/// exactly: this type defines the wire form on read, and a divergence here is a
/// silent rename.
#[derive(Deserialize)]
#[serde(tag = "scheme", rename_all = "camelCase")]
enum Wire {
    #[serde(rename_all = "camelCase")]
    Gs1 { gtin: Gtin },
    #[serde(rename_all = "camelCase")]
    IdentificationLink { url: String },
    #[serde(rename_all = "camelCase")]
    Did { did: String },
}

impl TryFrom<Wire> for ProductIdentifier {
    type Error = ProductIdentifierError;

    fn try_from(wire: Wire) -> Result<Self, Self::Error> {
        match wire {
            // `Gtin` has already validated its own check digit by this point.
            Wire::Gs1 { gtin } => Ok(Self::gs1(gtin)),
            Wire::IdentificationLink { url } => match check_web_url(&url) {
                // The two printable-ASCII rejections are for issuing, not for
                // reading. See the note on `Wire`.
                Ok(())
                | Err(WebUrlRejection::NonAsciiHost(_) | WebUrlRejection::NotPrintableAscii(_)) => {
                    Ok(Self::IdentificationLink { url })
                }
                Err(WebUrlRejection::NotAnAbsoluteWebUrl) => {
                    Err(ProductIdentifierError::NotAWebUrl(url))
                }
            },
            Wire::Did { did } => Self::did(&did),
        }
    }
}
