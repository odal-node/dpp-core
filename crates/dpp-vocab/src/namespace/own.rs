//! The namespace we coin our own identifiers in.

/// The namespace this project coins its own identifiers in.
///
/// An identifier under this prefix names **our** concept and needs no
/// provenance record. Anything outside it claims another organisation's
/// vocabulary and is permitted only by a verified record in
/// [`crate::register`].
///
/// # Why an own-namespace identifier is not a lesser option
///
/// `urn:odal-node:aas:property:repairability-score:1.0` says *"this is our
/// concept"*, truthfully. A wrong ECLASS IRDI in its place says *"this is
/// ECLASS's concept"*, falsely, to a machine, in the format most likely to be
/// consumed without a human ever looking at it. Declining to make a claim
/// beats making one we cannot support.
///
/// # Pending duplication
///
/// `dpp-aas` defines this same constant in `src/semantic_ids/mod.rs`. That copy
/// predates this crate and is removed when the consumers migrate; until then the
/// two must not be allowed to disagree, which is what the equality test in
/// `dpp-tests` is for. Two definitions of where the boundary sits is exactly the
/// condition under which a gate and the thing it guards drift apart.
pub const OWN_NAMESPACE: &str = "urn:odal-node:";

/// The namespace our **JSON-LD** terms expand into — the `dpp:` prefix.
///
/// A second form of the same ownership claim, in the syntax JSON-LD requires.
/// [`OWN_NAMESPACE`] is a URN and cannot serve here: a JSON-LD prefix IRI is
/// concatenated with the term to form an absolute IRI, and `urn:odal-node:` +
/// `product_group` is not one a consumer can do anything with.
///
/// # Why `is_own` deliberately does not cover this
///
/// [`is_own`](super::is_own) governs AAS `semanticId` values, where the URN form
/// is the only one this project mints. Widening it to admit an `https://` prefix
/// would make every `https://` semanticId — including another authority's —
/// one character of hostname away from being classified as ours, in the check
/// whose entire job is to refuse exactly that. The two forms are kept apart on
/// purpose, and a caller that means the JSON-LD one names it.
///
/// Not dereferenced. A prefix IRI names a vocabulary; it is not fetched during
/// expansion, so this URL carries no obligation to resolve — unlike a string
/// entry in an `@context` array, which does.
pub const OWN_JSONLD_NAMESPACE: &str = "https://schema.odal-node.io/dpp#";

/// Whether `iri` is one of our own identifiers.
///
/// ```
/// use dpp_vocab::namespace::is_own;
///
/// assert!(is_own("urn:odal-node:aas:property:product-name:1.0"));
/// assert!(!is_own("https://admin-shell.io/idta/nameplate/3/0/Nameplate"));
/// ```
#[must_use]
pub fn is_own(iri: &str) -> bool {
    iri.starts_with(OWN_NAMESPACE)
}
