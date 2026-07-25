use std::collections::HashSet;

use dpp_domain::Audience;

/// Registry of issuer DIDs authorised to issue credentials for each audience.
///
/// For MVP single-tenant, this is a static allow-list loaded from operator
/// configuration (see [`StaticTrustedIssuers`]). Use [`AllowAllIssuers`] only
/// in tests or pre-configuration bootstrapping — never in production.
pub trait TrustedIssuerRegistry: Send + Sync {
    fn is_trusted_for_audience(&self, issuer_did: &str, audience: Audience) -> bool;
}

/// Configuration-driven allow-list implementation of [`TrustedIssuerRegistry`].
///
/// **Issuer trust is hierarchical; data visibility is not.** An issuer trusted
/// to attest an authority is also trusted to attest a legitimate interest — the
/// harder attestation implies the easier one. That says nothing about what the
/// resulting credential can *see*: [`Audience::may_see`] withholds Annex XIII
/// point 4 from authorities regardless of who vouched for them. Do not use this
/// hierarchy as evidence that audiences are ordered.
pub struct StaticTrustedIssuers {
    legitimate_interest_dids: HashSet<String>,
    authority_dids: HashSet<String>,
}

impl StaticTrustedIssuers {
    pub fn new(
        legitimate_interest_dids: impl IntoIterator<Item = impl Into<String>>,
        authority_dids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            legitimate_interest_dids: legitimate_interest_dids
                .into_iter()
                .map(Into::into)
                .collect(),
            authority_dids: authority_dids.into_iter().map(Into::into).collect(),
        }
    }

    /// Single DID trusted for every audience (e.g. the operator's own issuer DID).
    pub fn single(trusted_did: impl Into<String>) -> Self {
        let did = trusted_did.into();
        Self {
            legitimate_interest_dids: HashSet::from([did.clone()]),
            authority_dids: HashSet::from([did]),
        }
    }
}

impl TrustedIssuerRegistry for StaticTrustedIssuers {
    fn is_trusted_for_audience(&self, issuer_did: &str, audience: Audience) -> bool {
        match audience {
            Audience::Public => true,
            Audience::LegitimateInterest => {
                self.legitimate_interest_dids.contains(issuer_did)
                    || self.authority_dids.contains(issuer_did)
            }
            Audience::Authority => self.authority_dids.contains(issuer_did),
            // Fail-closed: an unmodelled future audience trusts no issuer until
            // it is explicitly handled.
            _ => false,
        }
    }
}

/// Trust registry that accepts any issuer DID — use in tests or single-operator
/// bootstrap only. In production, supply a [`StaticTrustedIssuers`] loaded from
/// operator configuration.
pub struct AllowAllIssuers;

impl TrustedIssuerRegistry for AllowAllIssuers {
    fn is_trusted_for_audience(&self, _issuer_did: &str, _audience: Audience) -> bool {
        true
    }
}
