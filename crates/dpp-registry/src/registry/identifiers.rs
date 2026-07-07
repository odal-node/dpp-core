//! The four ESPR Article 13 persistent identifiers: product, product item,
//! facility, and economic operator.
//!
//! Grouped in one file rather than split per-type: they are one vocabulary
//! with symmetric shape (scheme + value + metadata), and share the
//! country-code / checksum validation helpers below.

use serde::{Deserialize, Serialize};

use super::error::RegistryValidationError;

fn validate_country_code(code: &str) -> Result<(), RegistryValidationError> {
    if code.is_empty() {
        return Ok(()); // unknown/not-yet-set is acceptable pre-go-live
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
    /// Checks the country code first, then applies a per-scheme structural or
    /// checksum check (see `validate_operator_scheme`).
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
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
