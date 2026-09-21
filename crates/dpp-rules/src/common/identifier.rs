//! EN 18219:2026 clause 5 identifier syntax, for the two tiers that check it.
//!
//! 🚨 **One home on purpose.** These predicates lived only in
//! `dpp_domain::identifier::ProductIdentifier`, and the plugin SDK grew its own
//! copy when the product-group payloads moved to `productIdentifier`. The copies
//! disagreed: the SDK accepted `https:///p/1` and `did:web: ` because it tested
//! a prefix and a non-empty remainder, while the domain tested the authority and
//! the W3C grammar. A plugin is the *first* thing to see product group data, so
//! the weaker of the two copies was the one on the outside.
//!
//! Kept dependency-free and `no_std` so the Wasm guest SDK can call the same
//! code the host does, rather than a second reading of the same clause.
//!
// LAYOUT-DEVIATION: rule 15 counts users among a bucket's siblings, inside one
// crate. This module's two users are `dpp-domain` and `dpp-plugin-sdk`, so the
// count it can see is zero and the sharing it is testing for is real but
// cross-crate. `dpp-rules` exists precisely to be depended on by both without
// either depending on the other, so a type shared that way has no in-crate
// sibling to count and cannot satisfy the rule as written.

/// The DID methods EN 18219 scheme 3 names.
///
/// The standard describes scheme 3 as Decentralized Identifiers and names these
/// three as the admissible methods, `did:web` being the lightweight non-DLT
/// option. Closed rather than open because an identifier exists to be followed:
/// a method no reader can resolve identifies nothing, and accepting one would
/// let a passport be created that is unreachable by design.
pub const DID_METHODS: [&str; 3] = ["web", "ethr", "ebsi"];

/// Why a candidate scheme 3 value is not an admissible DID.
///
/// Three variants rather than a `bool` because the callers report differently:
/// the domain has an error type per case, and a plugin turns them into one
/// field message. Neither should have to re-derive which case it hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DidRejection<'a> {
    /// Not `did:<method>:<method-specific-id>`, or the id breaks the W3C DID
    /// v1.0 clause 3.1 grammar.
    Malformed,
    /// Well-formed, but the method is outside the closed set clause 5 names.
    /// Carries the method as read, so a caller naming it in an error does not
    /// have to take the value apart a second time.
    UnsupportedMethod(&'a str),
    /// A named method with nothing after it — `did:web:` identifies no one.
    EmptyMethodId,
}

/// An absolute `http`/`https` URL with a host, as EN 18219 scheme 2 requires.
///
/// 🚨 The authority ends at the first `/`, `?` or `#` — it is not simply
/// "whatever follows the scheme". `https:///p/1` and `https://?q` each leave a
/// non-empty remainder and no host whatsoever, so testing that remainder for
/// emptiness accepted two values nothing can resolve.
///
/// This is a shape check, not a conformance claim: scheme 2's format is
/// specified by EN IEC 61406-1/-2 and those rules are **not** applied here.
#[must_use]
pub fn is_absolute_web_url(url: &str) -> bool {
    let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !authority.is_empty() && !url.contains(char::is_whitespace)
}

/// A DID under one of the methods [`DID_METHODS`] names.
///
/// # Errors
///
/// [`DidRejection`], naming which of the three ways the value failed.
pub fn check_did(did: &str) -> Result<(), DidRejection<'_>> {
    let rest = did.strip_prefix("did:").ok_or(DidRejection::Malformed)?;
    let (method, method_id) = rest.split_once(':').ok_or(DidRejection::Malformed)?;
    if !DID_METHODS.contains(&method) {
        return Err(DidRejection::UnsupportedMethod(method));
    }
    if method_id.is_empty() {
        return Err(DidRejection::EmptyMethodId);
    }
    if !is_method_specific_id(method_id) {
        return Err(DidRejection::Malformed);
    }
    Ok(())
}

/// W3C DID v1.0 clause 3.1: `method-specific-id = *( *idchar ":" ) 1*idchar`.
///
/// Colon-separated segments, of which only the last must be non-empty. Checked
/// because the shape is the whole claim the value makes — one carrying a raw
/// space or a truncated `%` escape is not a DID with a formatting blemish, it is
/// a string no resolver will accept, pointing at no passport.
fn is_method_specific_id(id: &str) -> bool {
    !id.is_empty() && !id.ends_with(':') && id.split(':').all(is_idchars)
}

/// `idchar = ALPHA / DIGIT / "." / "-" / "_" / pct-encoded`, where
/// `pct-encoded = "%" HEXDIG HEXDIG`.
fn is_idchars(segment: &str) -> bool {
    let mut chars = segment.chars();
    while let Some(c) = chars.next() {
        let ok = match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => true,
            '%' => matches!(
                (chars.next(), chars.next()),
                (Some(hi), Some(lo)) if hi.is_ascii_hexdigit() && lo.is_ascii_hexdigit()
            ),
            _ => false,
        };
        if !ok {
            return false;
        }
    }
    true
}
