//! Link descriptors and the content-negotiation algorithm.

use dpp_domain::{Audience, Disclosure};
use serde::{Deserialize, Serialize};

use super::media_type::DppMediaType;
use super::request::ResolutionRequest;
use super::vocabulary::Gs1LinkType;

/// Describes one available representation of a DPP resource.
///
/// A resolver builds a list of these for each product, and the negotiation
/// logic selects the best match for the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDescriptor {
    /// The URL of this representation.
    pub href: String,
    /// Link type (GS1 vocabulary).
    pub link_type: Gs1LinkType,
    /// Media type served at this URL.
    pub media_type: DppMediaType,
    /// Disclosure class of this resource — which audiences may view it.
    pub disclosure: Disclosure,
    /// Human-readable title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// ISO 639-1 language code (e.g., `"en"`, `"de"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// Negotiate the best link descriptor for a given request.
///
/// Priority:
/// 1. Match link type exactly (if specified).
/// 2. Among matches, prefer the requested media type.
/// 3. Filter by audience (return only resources the caller may see).
/// 4. If nothing matches, return `None`.
pub fn negotiate<'a>(
    available: &'a [LinkDescriptor],
    request: &ResolutionRequest,
) -> Option<&'a LinkDescriptor> {
    let audience = request.audience.unwrap_or(Audience::Public);

    // Filter by audience
    let accessible: Vec<&LinkDescriptor> = available
        .iter()
        .filter(|d| audience.may_see(d.disclosure))
        .collect();

    if accessible.is_empty() {
        return None;
    }

    // If link type is specified, filter by it; no accessible match → None
    let by_link_type: Vec<&LinkDescriptor> = if let Some(ref lt) = request.link_type {
        let matched: Vec<_> = accessible
            .iter()
            .filter(|d| d.link_type == *lt)
            .copied()
            .collect();
        if matched.is_empty() {
            return None;
        }
        matched
    } else {
        accessible
    };

    let candidates = &by_link_type;

    // If media type is specified, prefer it
    if let Some(ref mt) = request.media_type
        && let Some(exact) = candidates.iter().find(|d| d.media_type == *mt)
    {
        return Some(exact);
    }

    // Return first available candidate
    candidates.first().copied()
}
