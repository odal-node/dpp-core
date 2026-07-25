//! Identity and access types: `Audience`, `Disclosure`, `SignedCredential`, and `PassportCredential`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Who is asking for passport data.
///
/// Regulation (EU) 2023/1542 Art. 77(2) names three audiences and assigns each
/// a set of Annex XIII data points:
///
/// | Audience | Annex XIII |
/// |---|---|
/// | (a) general public | 1 |
/// | (b) notified bodies, market surveillance authorities, the Commission | 2 and 3 |
/// | (c) persons with a legitimate interest | 2 and 4 |
///
/// **This is a lattice, not a ranking.** Point 3 (conformity test reports) is
/// authority-only; point 4 (individual-item use history) is
/// legitimate-interest-only. Neither audience contains the other, so no integer
/// ordering can express the assignment: any `>=` comparison necessarily either
/// hands authorities the individual-item data Art. 77(2)(b) withholds, or hides
/// point-2 data from someone entitled to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Audience {
    /// Anyone, with no credential. Art. 77(2)(a).
    Public,
    /// A repairer, remanufacturer, second-life operator or recycler holding a
    /// credential that proves the interest. Art. 77(2)(c).
    LegitimateInterest,
    /// Notified body, market surveillance authority, customs, or the
    /// Commission. Art. 77(2)(b).
    Authority,
}

/// How restricted a field is — the counterpart to [`Audience`].
///
/// Named for the Annex XIII point each class corresponds to, and kept
/// sector-agnostic so non-battery sectors reuse the same vocabulary.
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
/// layer's `SectorAccessPolicy::passport_default()` both read it, because they
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

impl Audience {
    /// Whether this audience may see a field of class `disclosure`.
    ///
    /// The whole Art. 77(2) assignment, in one table.
    #[must_use]
    pub const fn may_see(self, disclosure: Disclosure) -> bool {
        matches!(
            (self, disclosure),
            (Self::Public, Disclosure::Public)
                | (
                    Self::LegitimateInterest,
                    Disclosure::Public | Disclosure::Restricted | Disclosure::Individual
                )
                | (
                    Self::Authority,
                    Disclosure::Public | Disclosure::Restricted | Disclosure::Conformity
                )
        )
    }
}

/// A W3C Verifiable Credential 2.0 envelope binding a DPP passport to its signed payload.
///
/// The cryptographic proof is in [`SignedCredential::jws`]; this struct provides
/// the structured VC context required for EUDI/EBSI interoperability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportCredential {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    /// Unique credential ID (`urn:uuid:…`) — generated fresh per signing call.
    pub id: String,
    /// DID of the signing issuer (`did:web:…`).
    pub issuer: String,
    /// Credential issuance timestamp (W3C VC 2.0 `validFrom`).
    pub valid_from: DateTime<Utc>,
    pub credential_subject: PassportCredentialSubject,
}

impl PassportCredential {
    /// W3C VCDM v2 base context — MUST be the first `@context` entry.
    pub const VC_BASE_CONTEXT: &'static str = "https://www.w3.org/ns/credentials/v2";
    /// Project-specific JSON-LD context for DPP passport credentials.
    pub const PASSPORT_CONTEXT: &'static str =
        "https://schema.odal-node.io/credentials/dpp-passport/v1";

    /// Construct a passport credential with the VCDM v2 base context and the
    /// `VerifiableCredential` base type guaranteed present, so a caller cannot
    /// emit a VC missing `https://www.w3.org/ns/credentials/v2`. `id`
    /// (`urn:uuid:` v7) and `valid_from` are generated fresh.
    #[must_use]
    pub fn new(issuer: String, credential_subject: PassportCredentialSubject) -> Self {
        Self {
            context: vec![Self::VC_BASE_CONTEXT.into(), Self::PASSPORT_CONTEXT.into()],
            credential_type: vec![
                "VerifiableCredential".into(),
                "DppPassportCredential".into(),
            ],
            id: format!("urn:uuid:{}", uuid::Uuid::now_v7()),
            issuer,
            valid_from: Utc::now(),
            credential_subject,
        }
    }
}

/// Claims about the DPP passport being attested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportCredentialSubject {
    /// `urn:uuid:{passport_id}` — the DPP passport being attested.
    pub id: String,
    /// SHA-256 hex digest of the RFC 8785 canonical payload bytes.
    pub payload_hash: String,
}

/// A DPP Verifiable Credential with its JWS proof signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedCredential {
    /// Structured W3C VC 2.0 passport credential.
    pub credential: PassportCredential,
    /// Compact JWS signature string (header.payload.signature).
    pub jws: String,
    /// The DID of the issuer (manufacturer or Odal on their behalf).
    pub issuer_did: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorities_do_not_see_individual_item_data() {
        // Art. 77(2)(b) assigns notified bodies, market surveillance and the
        // Commission Annex XIII points 2 and 3 — not point 4. This is the case
        // an ordinal tier cannot express, and the reason the model changed.
        assert!(!Audience::Authority.may_see(Disclosure::Individual));
        assert!(Audience::LegitimateInterest.may_see(Disclosure::Individual));
    }

    #[test]
    fn legitimate_interest_does_not_see_conformity_evidence() {
        // Point 3 (test reports) is authority-only under Art. 77(2)(b).
        assert!(!Audience::LegitimateInterest.may_see(Disclosure::Conformity));
        assert!(Audience::Authority.may_see(Disclosure::Conformity));
    }

    #[test]
    fn neither_non_public_audience_contains_the_other() {
        // The defining property of a lattice: if either audience were a superset
        // of the other, an ordinal ranking would suffice and this type would be
        // unnecessary. Each sees something the other does not.
        let all = [
            Disclosure::Public,
            Disclosure::Restricted,
            Disclosure::Conformity,
            Disclosure::Individual,
        ];
        let authority_only = all
            .iter()
            .any(|d| Audience::Authority.may_see(*d) && !Audience::LegitimateInterest.may_see(*d));
        let interest_only = all
            .iter()
            .any(|d| Audience::LegitimateInterest.may_see(*d) && !Audience::Authority.may_see(*d));
        assert!(authority_only && interest_only);
    }

    #[test]
    fn point_two_is_shared_and_public_is_universal() {
        for audience in [
            Audience::Public,
            Audience::LegitimateInterest,
            Audience::Authority,
        ] {
            assert!(audience.may_see(Disclosure::Public));
        }
        // Annex XIII point 2 goes to both non-public audiences.
        assert!(Audience::LegitimateInterest.may_see(Disclosure::Restricted));
        assert!(Audience::Authority.may_see(Disclosure::Restricted));
        assert!(!Audience::Public.may_see(Disclosure::Restricted));
    }

    #[test]
    fn public_sees_nothing_restricted() {
        for class in [
            Disclosure::Restricted,
            Disclosure::Conformity,
            Disclosure::Individual,
        ] {
            assert!(!Audience::Public.may_see(class));
        }
    }

    #[test]
    fn new_passport_credential_guarantees_vc_base_context_and_type() {
        let vc = PassportCredential::new(
            "did:web:issuer.example.com".into(),
            PassportCredentialSubject {
                id: "urn:uuid:00000000-0000-0000-0000-000000000000".into(),
                payload_hash: "deadbeef".into(),
            },
        );
        // VCDM v2 requires the base context to be the first @context entry.
        assert_eq!(
            vc.context.first().map(String::as_str),
            Some(PassportCredential::VC_BASE_CONTEXT)
        );
        assert!(
            vc.credential_type
                .contains(&"VerifiableCredential".to_string())
        );
        assert!(vc.id.starts_with("urn:uuid:"));
    }
}
