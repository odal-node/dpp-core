//! [`AssetIdentity`] — what the AAS shell says the asset *is*.
//!
//! # Why this is a type and not a `&str`
//!
//! The builder used to take `gtin: &str` and write it into two places that
//! both assert something about it: a `specificAssetId` **named** `gtin`, and a
//! `globalAssetId` of the form `urn:odal-node:product:{value}`. Neither
//! assertion was checked, and while every product carried a GTIN both happened
//! to be true.
//!
//! EN 18219 clause 5 admits two self-issuing schemes that have no GTIN, so
//! both became false for exactly the passports the identifier work exists to
//! enable. Measured before this type existed:
//!
//! ```text
//! specificAssetIds = ["gtin=did:web:acme.example.com:b:1", …]
//! globalAssetId    = urn:odal-node:product:did:web:acme.example.com:b:1
//! ```
//!
//! A DID filed under a GS1 label, inside a URN whose namespace-specific string
//! is another URI scheme. An AAS Environment is a document an integrator's
//! toolchain treats as authoritative, so neither is a cosmetic fault.
//!
//! # Why the parameter is not simply read off the passport
//!
//! It cannot be. The asset identity is the **caller's**: a file export, an
//! AASX package assembled for a reporting authority, or an unsold-goods
//! disclosure — which identifies a reporting period rather than a trade item
//! and has no product identifier at all — each need an identity the passport
//! does not carry.
//!
//! So the identity is self-describing instead. [`Product`](Self::Product)
//! derives its own label and needs no supervision;
//! [`Named`](Self::Named) keeps the escape hatch but has to be **asked for by
//! name**, which is the point: a caller supplying an internal key is making a
//! visible choice rather than inheriting a default that happens to be wrong.

use dpp_domain::identifier::ProductIdentifier;

use crate::builder::AasError;

/// The three `specificAssetId` names a [`ProductIdentity`](AssetIdentity)
/// derives, which a caller-chosen one may therefore not use.
///
/// 🚨 Reserved because the escape hatch would otherwise reproduce the exact
/// defect this type removes: `Named { name: "gtin", value: <a DID> }` is the
/// old behaviour, spelled out. A caller-chosen key exists for identities that
/// are *not* the product identifier, so it never has a legitimate reason to
/// claim one of these names.
pub const RESERVED_ASSET_ID_NAMES: [&str; 3] = ["gtin", "identificationLink", "did"];

/// What the shell's `globalAssetId` and `specificAssetIds` describe.
#[derive(Debug, Clone, Copy)]
pub enum AssetIdentity<'a> {
    /// The passport's own EN 18219 clause 5 identifier.
    ///
    /// The `specificAssetId` name is derived from
    /// [`ProductIdentifier::value_kind`], so the label cannot disagree with the
    /// value it labels.
    Product(&'a ProductIdentifier),
    /// A caller-chosen asset key, for an asset the passport's identifier does
    /// not name — a file export, an AASX package, a non-trade-item asset.
    Named {
        /// The `specificAssetId` name. May not be one of
        /// [`RESERVED_ASSET_ID_NAMES`].
        name: &'a str,
        /// The value, used verbatim.
        value: &'a str,
    },
}

impl<'a> AssetIdentity<'a> {
    /// The passport's own product identifier, when it has one.
    ///
    /// The common case in one call — and `None` is the whole point of the
    /// signature. A passport without an identifier is not an error and not a
    /// reason to invent one: an unsold-goods report identifies a reporting
    /// period rather than a trade item, and an unmodelled product group is
    /// reduced to its discriminant before any projection runs. Both give
    /// `None`, and the caller has to say what the asset is instead — which is
    /// exactly the decision that used to be made silently by whatever string
    /// happened to be passed.
    #[must_use]
    pub fn from_passport(passport: &'a dpp_domain::Passport) -> Option<Self> {
        passport
            .product_group_data
            .as_ref()
            .and_then(dpp_domain::ProductGroupData::product_identifier)
            .map(Self::Product)
    }

    /// The `specificAssetId` name for this identity.
    ///
    /// # Errors
    ///
    /// [`AasError::ReservedAssetIdName`] if a [`Named`](Self::Named) identity
    /// claims a name a scheme derives.
    pub fn asset_id_name(&self) -> Result<&str, AasError> {
        match self {
            Self::Product(id) => Ok(id.value_kind()),
            Self::Named { name, .. } => {
                if RESERVED_ASSET_ID_NAMES.contains(name) {
                    return Err(AasError::ReservedAssetIdName((*name).to_owned()));
                }
                if name.trim().is_empty() {
                    return Err(AasError::ReservedAssetIdName(String::from(
                        "<empty>: an asset id name must name something",
                    )));
                }
                Ok(name)
            }
        }
    }

    /// The identity as written.
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::Product(id) => id.as_str(),
            Self::Named { value, .. } => value,
        }
    }

    /// Check that this identity can form an AAS `Identifier` at all.
    ///
    /// 🚨 **Nothing downstream does.** The AAS metamodel types `globalAssetId`
    /// as an `Identifier`, but the reference implementation verifies only its
    /// length — measured against `aas-core3.0` 1.1.4, which **accepts**
    /// `urn:odal-node:product:asset 1` (a raw space), `urn:odal-node:product:`
    /// (no namespace-specific string at all) and an embedded tab. So this is
    /// the last place a value that cannot be an identifier can be stopped.
    ///
    /// Only [`Named`](Self::Named) needs checking. A
    /// [`Product`](Self::Product) value comes from `ProductIdentifier`, whose
    /// three arms already refuse whitespace: a GTIN is fourteen digits, a
    /// scheme 2 link is an absolute URL rejected for any whitespace, and a DID
    /// must satisfy the W3C `idchar` grammar.
    ///
    /// **Rejected rather than percent-encoded.** Encoding would make the
    /// shell's identifier a different string from the one the caller supplied,
    /// which is its own kind of false statement — and a caller choosing an
    /// asset key is in a position to choose a usable one.
    ///
    /// # Errors
    ///
    /// [`AasError::ReservedAssetIdName`] if the name is one a scheme derives,
    /// and [`AasError::UnusableAssetIdentity`] if the value cannot appear in a
    /// URI.
    pub fn validate(&self) -> Result<(), AasError> {
        self.asset_id_name()?;
        let Self::Named { value, .. } = self else {
            return Ok(());
        };
        if value.trim().is_empty() {
            return Err(AasError::UnusableAssetIdentity(String::from(
                "an empty asset identity names nothing",
            )));
        }
        if let Some(bad) = value.chars().find(|c| c.is_whitespace() || c.is_control()) {
            return Err(AasError::UnusableAssetIdentity(format!(
                "asset identity {value:?} contains {bad:?}, which cannot appear in a URI"
            )));
        }
        Ok(())
    }

    /// The shell's `globalAssetId`.
    ///
    /// 🚨 **A value that is already a URI is used as-is.** The AAS
    /// `globalAssetId` is an identifier in its own right, and a clause 5
    /// scheme 2 link or scheme 3 DID already is one — wrapping either in
    /// `urn:odal-node:product:` produces a URN whose namespace-specific string
    /// is another scheme, and in the `urn:uuid:` case a URN nested inside a
    /// URN. Only a bare GTIN, which is not a URI at all, needs the wrapper.
    ///
    /// For a [`Named`](Self::Named) identity the same rule is applied by
    /// inspection, since the caller may legitimately supply either.
    #[must_use]
    pub fn global_asset_id(&self) -> String {
        let value = self.value();
        let already_a_uri = match self {
            Self::Product(id) => id.is_uri(),
            Self::Named { .. } => looks_like_a_uri(value),
        };
        if already_a_uri {
            value.to_owned()
        } else {
            format!("urn:odal-node:product:{value}")
        }
    }
}

/// A conservative "is this already an absolute URI" test for a caller-supplied
/// value.
///
/// Deliberately narrow: it asks whether there is a scheme before the first
/// `:`, per RFC 3986 `scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`.
/// A value that is not one gets wrapped, which is the safe direction — wrapping
/// a non-URI produces a valid URN, while failing to wrap one produces a
/// `globalAssetId` that is not an identifier at all.
fn looks_like_a_uri(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };
    !rest.is_empty()
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}
