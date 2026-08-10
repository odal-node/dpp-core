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
