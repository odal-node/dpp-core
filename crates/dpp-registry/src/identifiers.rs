//! The persistent identifiers a DPP carries: product, product item, facility,
//! and economic operator.
//!
//! **Where these come from.** ESPR (EU) 2024/1781 **Annex III** ("Digital
//! product Passport", referred to in Articles 9 to 12) is what specifies the
//! data elements, including the unique product identifier (point (b)), the
//! GTIN (point (c)), unique operator identifiers (points **(g), (h) and (k)**)
//! and unique facility identifiers (point (i)). **Art. 12** imposes standards
//! conformity on the operator and facility identifiers; **Art. 13** sets up the
//! registry that *stores* them. Annex III names three identifier *types* —
//! product, operator, facility — so "the four Article 13 identifiers" was wrong
//! on both the count and the article, and is corrected here.
//!
//! **Point (k) belongs in that enumeration and was missing.** Annex III's second
//! paragraph names it explicitly alongside (g) and (h): "the unique operator
//! identifiers referred to in points (g), (h) and (k) … shall, where relevant
//! for the products concerned, comply with standards ISO/IEC 15459-1:2014 …".
//! Point (k) is the EU-established operator answerable for the Art. 4 of
//! Regulation (EU) 2019/1020 or Art. 16 of Regulation (EU) 2023/988 tasks, so
//! there are **three** classes of operator identifier, not two.
//!
//! ⚠️ That has a consequence this module does not yet carry: (g) and (h) travel
//! as [`OperatorIdentifier`], while point (k)'s identifier is a bare `String` on
//! `dpp_domain::operator::ResponsibleOperator`. Annex III subjects all three to
//! the same conformity rule, and only two of them are shaped to express it.
//! Closing that is a persisted-shape change, not a doc fix.
//!
//! [`ProductItemIdentifier`] is not a fourth Annex III class: it is the product
//! identifier at item granularity, which Annex III point (b) leaves to "the
//! level indicated in the applicable delegated act". It is modelled separately
//! because serialised products need a distinct shape (batch/lot alongside the
//! serial), not because the Regulation lists it apart.
//!
//! [`ServiceProviderReference`] is Annex III **point (l)**, and is here for the
//! same reason: it names a party, with the same scheme/value/metadata shape.
//! It is deliberately *not* called an identifier — see the type.
//!
//! Grouped in one file rather than split per-type: they are one vocabulary
//! with symmetric shape (scheme + value + metadata), and share the
//! country-code / checksum validation helpers below.

use serde::{Deserialize, Serialize};

use super::error::RegistryValidationError;

fn validate_country_code(code: &str) -> Result<(), RegistryValidationError> {
    if code.is_empty() {
        // `country` is a non-optional Annex III field; an empty value is a
        // missing mandatory identifier, not an acceptable "unknown".
        return Err(RegistryValidationError::MissingRequiredField(
            "country".into(),
        ));
    }
    if dpp_rules::country_code_valid(code) {
        Ok(())
    } else {
        Err(RegistryValidationError::InvalidCountryCode {
            code: code.to_owned(),
        })
    }
}

/// Unique product identifier — identifies the product model/type.
///
/// Typically a GTIN-14 or similar standardised product code. The EU registry
/// uses this to group all items of the same product under one entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductIdentifier {
    /// The identifier scheme (e.g. `"gtin"`, `"gln"`, `"did"`).
    pub scheme: String,
    /// The identifier value (e.g. `"09506000134352"`).
    pub value: String,
    /// Optional human-readable label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl ProductIdentifier {
    /// Validate the identifier by scheme.
    ///
    /// - `"gtin"`: value must be a structurally valid GTIN-14 (14 digits, mod-10 check).
    /// - Other schemes: no structural validation (formats vary; validate when EU spec is published).
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        if self.scheme == "gtin" {
            dpp_domain::Gtin::parse(&self.value)
                .map(|_| ())
                .map_err(|e| RegistryValidationError::InvalidGtin {
                    value: self.value.clone(),
                    reason: format!("{e:?}"),
                })?;
        }
        Ok(())
    }
}

/// Product item identifier — identifies an individual serialised unit.
///
/// For serialised products (batteries with serial numbers, individual garments
/// with SGTIN), this distinguishes one physical item from another.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductItemIdentifier {
    /// The identifier scheme (e.g. `"sgtin"`, `"serial"`, `"batch+serial"`).
    pub scheme: String,
    /// The identifier value.
    pub value: String,
    /// Batch or lot number, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
}

impl ProductItemIdentifier {
    /// Validate the item identifier: `scheme` and `value` must both be
    /// non-empty. Scheme-specific structural checks (SGTIN layout, etc.) are
    /// deferred until the EU fixes item-identifier formats.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        if self.scheme.is_empty() {
            return Err(RegistryValidationError::MissingRequiredField(
                "itemId.scheme".into(),
            ));
        }
        if self.value.is_empty() {
            return Err(RegistryValidationError::MissingRequiredField(
                "itemId.value".into(),
            ));
        }
        Ok(())
    }
}

/// Facility identifier — identifies the manufacturing or assembly facility.
///
/// Used for market surveillance to trace products back to their physical
/// origin. Must match the facility registered under the economic operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FacilityIdentifier {
    /// The facility code scheme (e.g. `"gln"`, `"lei"`, `"national"`).
    pub scheme: String,
    /// The facility identifier value.
    pub value: String,
    /// Human-readable facility name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// ISO 3166-1 alpha-2 country code where the facility is located.
    pub country: String,
    /// Street address or location description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

impl FacilityIdentifier {
    /// Validate the facility identifier.
    ///
    /// Checks the country code and, when `scheme == "gln"`, that the value is a
    /// structurally valid GS1 GLN (13 digits, mod-10 check). Other schemes
    /// (`"lei"`, `"national"`, …) are not structurally verified here.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        validate_country_code(&self.country)?;
        if self.scheme == "gln" {
            dpp_domain::Gln::parse(&self.value)
                .map(|_| ())
                .map_err(|e| RegistryValidationError::InvalidGln {
                    value: self.value.clone(),
                    reason: format!("{e:?}"),
                })?;
        }
        Ok(())
    }
}

/// Economic operator identifier — identifies the responsible legal entity.
///
/// This is the entity accountable for the DPP under ESPR. When a transfer
/// of responsibility occurs, the registry record is updated to reflect the
/// new operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperatorIdentifier {
    /// The operator identifier scheme (e.g. `"vat"`, `"lei"`, `"eori"`, `"did"`).
    pub scheme: String,
    /// The identifier value (e.g. EU VAT number, LEI code).
    pub value: String,
    /// Legal entity name.
    pub name: String,
    /// ISO 3166-1 alpha-2 country of registration.
    pub country: String,
    /// DID of the operator, if available (for VC-based authentication).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
}

impl OperatorIdentifier {
    /// Validate the operator identifier.
    ///
    /// Checks that `name` is present, then the country code, then applies a
    /// per-scheme structural or checksum check (see `validate_operator_scheme`).
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        if self.name.is_empty() {
            return Err(RegistryValidationError::MissingRequiredField(
                "operatorId.name".into(),
            ));
        }
        // An identifier with no scheme is not identifiable: the value alone does
        // not say whether it is a VAT number, an LEI or a DID. Refused here
        // because the per-scheme check below accepts any unrecognised scheme —
        // including the empty one — so without this an unscheme'd identifier
        // would pass validation and be submitted as if it were well-formed.
        if self.scheme.trim().is_empty() {
            return Err(RegistryValidationError::MissingRequiredField(
                "operatorId.scheme".into(),
            ));
        }
        validate_country_code(&self.country)?;
        validate_operator_scheme(&self.scheme, &self.value)
    }
}

/// Per-scheme structural/checksum validation of an economic-operator identifier.
///
/// - `lei`  — ISO 17442: 20 alphanumerics with an ISO 7064 MOD 97-10 check digit.
/// - `eori` — 2-letter country prefix + 1..=15 alphanumerics.
/// - `vat`  — 2-letter country prefix + alphanumerics (member-state check digits
///   are **not** enforced; formats vary).
/// - `duns` — exactly 9 digits.
/// - `did`  — a DID; not structurally verified here (the crypto layer validates it).
/// - any other scheme — accepted but **not** structurally verified.
fn validate_operator_scheme(scheme: &str, value: &str) -> Result<(), RegistryValidationError> {
    let ok = match scheme {
        "lei" => lei_checksum_valid(value),
        "eori" => has_country_prefix(value, 15),
        "vat" => has_country_prefix(value, usize::MAX),
        "duns" => value.len() == 9 && value.bytes().all(|b| b.is_ascii_digit()),
        // "did" and unknown schemes are accepted without structural verification.
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(RegistryValidationError::InvalidOperatorId {
            scheme: scheme.to_owned(),
            value: value.to_owned(),
        })
    }
}

/// ISO 7064 MOD 97-10 check over a 20-character LEI (ISO 17442).
///
/// Letters map A→10 … Z→35; the streamed value mod 97 must equal 1.
fn lei_checksum_valid(s: &str) -> bool {
    if s.len() != 20
        || !s
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return false;
    }
    let mut rem: u32 = 0;
    for b in s.bytes() {
        let v = if b.is_ascii_digit() {
            u32::from(b - b'0')
        } else {
            u32::from(b - b'A') + 10
        };
        // A letter contributes two decimal positions (10–35); a digit one.
        rem = if v >= 10 {
            (rem * 100 + v) % 97
        } else {
            (rem * 10 + v) % 97
        };
    }
    rem == 1
}

/// Two-letter country prefix followed by 1..=`max_body` alphanumeric characters.
fn has_country_prefix(s: &str, max_body: usize) -> bool {
    let b = s.as_bytes();
    s.len() >= 3
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_uppercase()
        && (s.len() - 2) <= max_body
        && s[2..]
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// The digital product passport service provider hosting the back-up copy —
/// ESPR Annex III point **(l)**.
///
/// ✅ COMPLIANCE-PIN: Regulation (EU) 2024/1781 Annex III(l), verbatim — *"the
/// reference of the digital product passport service provider hosting the
/// back-up copy of the digital product passport."* Commission Implementing
/// Regulation (EU) 2026/1778 **Art. 8(9)(c)** makes it registration data the
/// Commission stores: *"where relevant, reference to the digital product
/// passport service provider"*.
///
/// # A reference, not an identifier, and the distinction is the act's
///
/// Annex III's second paragraph subjects the data carrier, the unique product
/// identifier of point (b), the operator identifiers of points (g), (h) and (k)
/// and the facility identifiers of point (i) to ISO/IEC 15459 conformity.
/// **Point (l) is not in that list.** No scheme is mandated for it, and
/// [Art. 2(32)](https://eur-lex.europa.eu/eli/reg/2024/1781/oj) imposes none on
/// the role either — a provider is *"an independent third-party authorised by
/// the economic operator"*, which is a legal relationship rather than a
/// registration.
///
/// So this carries a **name**, which is what makes the reference a reference,
/// and an optional scheme/value pair for providers that do hold a registry
/// identifier. Requiring a scheme would invent a conformity rule the annex
/// deliberately does not impose.
///
/// 🚨 **A provider is verified, and verification still yields no identifier to
/// reuse.** IR (EU) 2026/1778 recital 4 names the role among value chain actors
/// — *"each economic operator and value chain actor (such as digital product
/// passport service provider, repairer, refurbisher, remanufacturer, recycler)
/// should be identified through a verification process"* — so **Art. 5** governs
/// it: a legal person obtains verified status by proving identity and
/// establishment with a qualified electronic seal or a qualified electronic
/// attestation of attributes. Art. 3(f) then has the registry holding *"a list
/// of verified digital product passport service providers"*.
///
/// That produces a **status evidenced by a seal**, not a namespace. Art. 5 names
/// the evidence and assigns nothing, so a design reaching for a registry-issued
/// provider identifier would still be inventing one — and Art. 5(4) caps
/// verified status at the expiry of the electronic identification means or three
/// years from verification, whichever comes first, so a reference records who
/// was named rather than asserting they are verified today.
///
/// # Why it is not the back-up URL
///
/// `RegistrationPayload::backup_url` is a location; this is the party. IR
/// 2026/1778 lists them in consecutive paragraphs — Art. 8(7)(e) confirms
/// *"the link to the back-up hosted by a digital product passport service
/// provider"*, Art. 8(9)(c) stores the reference to the provider — and a URL
/// cannot stand in for the other. Two providers can serve from one domain, one
/// provider can serve from many, and a provider can be engaged with no link
/// declared to the registry at all.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProviderReference {
    /// The provider's legal name.
    ///
    /// Required: a reference that names nobody references nothing, and the name
    /// is the only element Annex III(l) can be read to compel.
    pub name: String,
    /// The identifier scheme, where the provider holds one — `"vat"`, `"lei"`,
    /// `"eori"`, `"duns"`, `"did"`, as for an economic operator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    /// The identifier value under [`scheme`](Self::scheme).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// ISO 3166-1 alpha-2 country of establishment, where known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

impl ServiceProviderReference {
    /// A reference carrying only the provider's name.
    ///
    /// The floor Annex III(l) sets, and lawful on its own because the annex
    /// mandates no scheme for point (l).
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            scheme: None,
            value: None,
            country: None,
        }
    }

    /// Validate the reference.
    ///
    /// # Errors
    ///
    /// [`RegistryValidationError::MissingRequiredField`] if the name is empty,
    /// or if a scheme and a value do not arrive together;
    /// [`RegistryValidationError::InvalidCountryCode`] if a country is present
    /// and is not ISO 3166-1 alpha-2; and whatever the per-scheme check refuses.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        if self.name.trim().is_empty() {
            return Err(RegistryValidationError::MissingRequiredField(
                "serviceProvider.name".into(),
            ));
        }
        // 🚨 All or nothing, for the reason `OperatorIdentifier::validate`
        // already gives: a value with no scheme does not say whether it is a VAT
        // number, an LEI or a DID, so it identifies nobody while looking as
        // though it does. A scheme with no value is the same defect mirrored.
        match (self.scheme.as_deref(), self.value.as_deref()) {
            (None, None) => {}
            (Some(scheme), Some(value)) => {
                if scheme.trim().is_empty() {
                    return Err(RegistryValidationError::MissingRequiredField(
                        "serviceProvider.scheme".into(),
                    ));
                }
                // 🚨 And the value, before the per-scheme check rather than
                // through it. `validate_operator_scheme` accepts `"did"` and
                // every unrecognised scheme without looking at the value, so a
                // blank one reached `Ok` — which made `Some("did")` with an
                // empty value pass while `Some("did")` with no value at all is
                // refused two arms down. The same absence, two answers.
                if value.trim().is_empty() {
                    return Err(RegistryValidationError::MissingRequiredField(
                        "serviceProvider.value".into(),
                    ));
                }
                validate_operator_scheme(scheme, value)?;
            }
            (Some(_), None) => {
                return Err(RegistryValidationError::MissingRequiredField(
                    "serviceProvider.value".into(),
                ));
            }
            (None, Some(_)) => {
                return Err(RegistryValidationError::MissingRequiredField(
                    "serviceProvider.scheme".into(),
                ));
            }
        }
        if let Some(country) = &self.country {
            validate_country_code(country)?;
        }
        Ok(())
    }
}
