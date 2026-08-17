//! Identity and access types: `Audience`, `Disclosure`, `SignedCredential`, and `PassportCredential`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

impl Disclosure {
    /// How many of the three audiences may see this class.
    ///
    /// Not an ordering of the lattice — there isn't one. It is the only totally
    /// ordered thing the lattice offers, which is what a deterministic tie-break
    /// needs.
    const fn audience_count(self) -> u8 {
        match self {
            // Everyone.
            Self::Public => 3,
            // Legitimate interest and authorities, not the public.
            Self::Restricted => 2,
            // Exactly one audience each, and not the same one.
            Self::Conformity | Self::Individual => 1,
        }
    }

    /// The more restrictive of two classes — the one fewer audiences may see.
    ///
    /// Exists so that an ambiguous lookup resolves the safe way and resolves it
    /// **identically every time**. Used by
    /// [`SectorAccessPolicy::disclosure_for_field`](crate::access::SectorAccessPolicy::disclosure_for_field)
    /// when two normalized-equal keys both match.
    ///
    /// `Conformity` and `Individual` are genuinely incomparable — Art. 77(2)
    /// gives each to one audience, and neither audience contains the other, so
    /// no class means "withheld from both". The tie-break returns `Individual`,
    /// which is a choice rather than a derivation, and it is only ever reached
    /// by a policy that declares one field name in both classes. That is an
    /// authoring error a schema cannot commit: `access::tests` rejects it at
    /// build time.
    #[must_use]
    pub const fn most_restrictive(self, other: Self) -> Self {
        if other.audience_count() <= self.audience_count() {
            other
        } else {
            self
        }
    }

    /// The stable wire token for this class, used to build a disclosure-set key.
    ///
    /// Deliberately not `Serialize`-derived: this string is baked into stored
    /// artefact keys, so it must be stable independently of any future serde
    /// attribute change on the enum.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Restricted => "restricted",
            Self::Conformity => "conformity",
            Self::Individual => "individual",
        }
    }
}

/// Every disclosure class, in the fixed order a [`disclosure_key`] uses.
///
/// Ordering is by Annex XIII point number, and it is part of the key format:
/// two nodes must produce byte-identical keys for the same set.
const DISCLOSURE_ORDER: &[Disclosure] = &[
    Disclosure::Public,
    Disclosure::Restricted,
    Disclosure::Conformity,
    Disclosure::Individual,
];

/// Name a set of disclosure classes: the classes' tokens in Annex XIII order,
/// joined with `+` — e.g. `public+restricted+individual`.
///
/// **This is how durable artefacts are keyed, and it must never be an audience
/// name.** ESPR uses a ~14-class actor vocabulary that is not battery's
/// three-audience lattice, and the delegated act mapping actors to data does not
/// exist yet. A signature or audit row keyed `"legitimateInterest"` would have
/// to be migrated the day that mapping lands; one keyed by the disclosure set it
/// actually covers keeps meaning exactly what it always meant, and a new actor
/// taxonomy becomes a new mapping onto the same keys.
#[must_use]
pub fn disclosure_key(classes: &[Disclosure]) -> String {
    DISCLOSURE_ORDER
        .iter()
        .filter(|c| classes.contains(c))
        .map(|c| c.token())
        .collect::<Vec<_>>()
        .join("+")
}

impl Audience {
    /// The disclosure classes this audience may see, in Annex XIII order.
    #[must_use]
    pub fn disclosure_set(self) -> Vec<Disclosure> {
        DISCLOSURE_ORDER
            .iter()
            .copied()
            .filter(|d| self.may_see(*d))
            .collect()
    }

    /// The [`disclosure_key`] for this audience's classes — the name under which
    /// a view served to it is signed and audited.
    ///
    /// Two audiences with the same class set would share a key, and that is
    /// correct: the artefact describes the data it covers, not who asked.
    #[must_use]
    pub fn disclosure_key(self) -> String {
        disclosure_key(&self.disclosure_set())
    }

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
    pub context: Vec<Value>,
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

    /// Inline JSON-LD term map for the DPP-specific terms this credential adds
    /// on top of the VCDM v2 base context: the credential type value and the
    /// one custom subject property (`payloadHash`).
    ///
    /// Inlined rather than hosted at a URL — a string entry in `@context` is
    /// fetched by the consumer at expansion time, and this crate does not host
    /// a context document. Same reasoning and `dpp:` prefix as
    /// `dpp_vc::jsonld::context::passport_context`; a prefix IRI names a
    /// vocabulary and is never dereferenced during expansion, so it carries no
    /// such obligation.
    fn dpp_terms() -> Value {
        json!({
            "dpp": "https://schema.odal-node.io/dpp#",
            "DppPassportCredential": "dpp:DppPassportCredential",
            "payloadHash": "dpp:payloadHash",
        })
    }

    /// Construct a passport credential with the VCDM v2 base context and the
    /// `VerifiableCredential` base type guaranteed present, so a caller cannot
    /// emit a VC missing `https://www.w3.org/ns/credentials/v2`. `id`
    /// (`urn:uuid:` v7) and `valid_from` are generated fresh.
    #[must_use]
    pub fn new(issuer: String, credential_subject: PassportCredentialSubject) -> Self {
        Self {
            context: vec![json!(Self::VC_BASE_CONTEXT), Self::dpp_terms()],
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

    /// The key names the *data*, not the asker. This is the property that lets a
    /// stored signature survive ESPR naming a different actor taxonomy, so it is
    /// asserted literally rather than round-tripped.
    #[test]
    fn a_disclosure_key_names_classes_never_an_audience() {
        assert_eq!(Audience::Public.disclosure_key(), "public");
        assert_eq!(
            Audience::LegitimateInterest.disclosure_key(),
            "public+restricted+individual"
        );
        assert_eq!(
            Audience::Authority.disclosure_key(),
            "public+restricted+conformity"
        );

        for audience in [
            Audience::Public,
            Audience::LegitimateInterest,
            Audience::Authority,
        ] {
            let key = audience.disclosure_key();
            for name in ["legitimate", "authority", "public_", "audience"] {
                assert!(
                    !key.contains(name) || key == "public",
                    "{key} leaks an audience name into a durable artefact key"
                );
            }
        }
    }

    /// Key construction is order-independent in its input and fixed in its
    /// output: two nodes handed the same set in different orders must produce
    /// byte-identical keys, or the same view signs under two names.
    #[test]
    fn a_disclosure_key_is_canonical_regardless_of_input_order() {
        let forward = disclosure_key(&[
            Disclosure::Public,
            Disclosure::Restricted,
            Disclosure::Individual,
        ]);
        let reversed = disclosure_key(&[
            Disclosure::Individual,
            Disclosure::Restricted,
            Disclosure::Public,
        ]);
        assert_eq!(forward, reversed);
        assert_eq!(forward, "public+restricted+individual");
        assert_eq!(disclosure_key(&[]), "");
    }

    /// The set an audience is keyed by must be exactly what `may_see` grants —
    /// if these drift, a view is signed under a key that overstates or
    /// understates what it contains.
    #[test]
    fn the_disclosure_set_agrees_with_may_see() {
        for audience in [
            Audience::Public,
            Audience::LegitimateInterest,
            Audience::Authority,
        ] {
            let set = audience.disclosure_set();
            for class in [
                Disclosure::Public,
                Disclosure::Restricted,
                Disclosure::Conformity,
                Disclosure::Individual,
            ] {
                assert_eq!(
                    set.contains(&class),
                    audience.may_see(class),
                    "{audience:?} / {class:?}: disclosure_set disagrees with may_see"
                );
            }
        }
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
            vc.context.first().and_then(Value::as_str),
            Some(PassportCredential::VC_BASE_CONTEXT)
        );
        assert!(
            vc.credential_type
                .contains(&"VerifiableCredential".to_string())
        );
        assert!(vc.id.starts_with("urn:uuid:"));
    }
}
