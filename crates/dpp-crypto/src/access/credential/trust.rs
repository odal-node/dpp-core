use std::collections::HashSet;

use dpp_domain::AccessTier;

/// Registry of issuer DIDs authorised to grant each access tier.
///
/// For MVP single-tenant, this is a static allow-list loaded from operator
/// configuration (see [`StaticTrustedIssuers`]). Use [`AllowAllIssuers`] only
/// in tests or pre-configuration bootstrapping — never in production.
pub trait TrustedIssuerRegistry: Send + Sync {
    fn is_trusted_for_tier(&self, issuer_did: &str, tier: AccessTier) -> bool;
}

/// Configuration-driven allow-list implementation of [`TrustedIssuerRegistry`].
///
/// A DID in `confidential_dids` is implicitly also trusted for Professional-tier
/// credentials — Confidential implies Professional.
pub struct StaticTrustedIssuers {
    professional_dids: HashSet<String>,
    confidential_dids: HashSet<String>,
}

impl StaticTrustedIssuers {
    pub fn new(
        professional_dids: impl IntoIterator<Item = impl Into<String>>,
        confidential_dids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            professional_dids: professional_dids.into_iter().map(Into::into).collect(),
            confidential_dids: confidential_dids.into_iter().map(Into::into).collect(),
        }
    }

    /// Single DID trusted for all tiers (e.g. the operator's own issuer DID).
    pub fn single(trusted_did: impl Into<String>) -> Self {
        let did = trusted_did.into();
        Self {
            professional_dids: HashSet::from([did.clone()]),
            confidential_dids: HashSet::from([did]),
        }
    }
}

impl TrustedIssuerRegistry for StaticTrustedIssuers {
    fn is_trusted_for_tier(&self, issuer_did: &str, tier: AccessTier) -> bool {
        match tier {
            AccessTier::Public => true,
            AccessTier::Professional => {
                self.professional_dids.contains(issuer_did)
                    || self.confidential_dids.contains(issuer_did)
            }
            AccessTier::Confidential => self.confidential_dids.contains(issuer_did),
            // Fail-closed: an unmodelled (future, more-sensitive) tier trusts
            // no issuer until it is explicitly handled.
            _ => false,
        }
    }
}

/// Trust registry that accepts any issuer DID — use in tests or single-operator
/// bootstrap only. In production, supply a [`StaticTrustedIssuers`] loaded from
/// operator configuration.
pub struct AllowAllIssuers;

impl TrustedIssuerRegistry for AllowAllIssuers {
    fn is_trusted_for_tier(&self, _issuer_did: &str, _tier: AccessTier) -> bool {
        true
    }
}
