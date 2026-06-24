//! Resolution request and HTTP `Accept`-header media-type negotiation.

use dpp_domain::AccessTier;

use super::media_type::DppMediaType;
use super::vocabulary::Gs1LinkType;

/// A parsed resolution request combining the Digital Link with negotiation hints.
#[derive(Debug, Clone)]
pub struct ResolutionRequest {
    /// The requested link type (from `?linkType=` query parameter).
    pub link_type: Option<Gs1LinkType>,
    /// The requested media type (from `Accept` header).
    pub media_type: Option<DppMediaType>,
    /// The access tier context (from authentication / credential).
    /// `None` means public access.
    pub access_tier: Option<AccessTier>,
}

impl ResolutionRequest {
    /// Build a resolution request from an HTTP `Accept` header string.
    ///
    /// Parses q-values per RFC 9110 §12.4 and selects the highest-priority
    /// non-wildcard media type. Wildcards (`*/*`, `application/*`) are
    /// deprioritised below explicit types at the same q-value.
    pub fn from_accept_header(accept: &str) -> Self {
        Self {
            link_type: None,
            media_type: parse_best_media_type(accept),
            access_tier: None,
        }
    }
}

/// Parse an RFC 9110 `Accept` header and return the highest-priority
/// non-wildcard media type, or `None` for `*/*`-only requests.
fn parse_best_media_type(accept: &str) -> Option<DppMediaType> {
    let mut entries: Vec<(String, f32)> = accept
        .split(',')
        .filter_map(|entry| {
            let mut parts = entry.trim().splitn(2, ';');
            let media = parts.next()?.trim().to_owned();
            if media.is_empty() {
                return None;
            }
            let q = parts
                .next()
                .and_then(|p| p.trim().strip_prefix("q="))
                .and_then(|q| q.parse::<f32>().ok())
                .unwrap_or(1.0);
            Some((media, q))
        })
        .collect();

    // Sort by q descending; at equal q, explicit types sort before wildcards.
    entries.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let a_wild = a.0.contains('*');
                let b_wild = b.0.contains('*');
                match (a_wild, b_wild) {
                    (false, true) => std::cmp::Ordering::Less,
                    (true, false) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            })
    });

    entries
        .iter()
        .filter(|(m, q)| *q > 0.0 && !m.contains('*'))
        .map(|(m, _)| DppMediaType::parse(m))
        .next()
}
