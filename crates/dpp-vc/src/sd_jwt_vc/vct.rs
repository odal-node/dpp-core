//! The `vct` claim — what a credential *is*, named so the name outlives us.

use dpp_domain::ProductGroup;

/// The authority name and date of the `tag:` URIs this crate mints.
///
/// Fixed, and never to be advanced. RFC 4151 clause 2.2: the date identifies a
/// period during which the authority held the name, and a tag minted under it
/// stays valid *"even if the domain name later changes hands"*. Bumping the year
/// would mint a second, unrelated namespace and make previously issued
/// credentials look like a different type.
const TAG_AUTHORITY: &str = "odal-node.io,2026";

/// The `vct` for a product group at a schema version.
///
/// # Why a `tag:` URI and not an HTTPS URL
///
/// `vct` must be a Collision-Resistant Name (draft-ietf-oauth-sd-jwt-vc-19
/// clause 2.2.2.1, referring to RFC 7515 clause 2). An HTTPS URL is one, and the
/// profile then permits a consumer to fetch Type Metadata from it. Three facts
/// make that the wrong trade here:
///
/// - **Retention outlives hosting.** A published passport is not rewritable and
///   has to remain readable for years. An HTTPS `vct` binds a DNS name into an
///   immutable artefact for that whole period, and a name that stops resolving
///   cannot be corrected in credentials already issued.
/// - **A dereferenceable `vct` puts a third party in every verification.** The
///   reason to issue a credential at all is that a holder can prove something
///   without the issuing node being reachable. A type identifier that must be
///   fetched from one host reintroduces exactly that dependency.
/// - **The type is not the issuer's.** Every operator issuing this credential
///   issues the *same* type. If each minted the identifier from its own domain,
///   two operators' credentials would disagree about what they are, and `vct`
///   would stop doing the one job it has.
///
/// A `tag:` URI (RFC 4151) is collision-resistant, requires no registration —
/// unlike a `urn:` NID, which would leave a registration debt nobody is going to
/// pay — and is **non-dereferenceable by design**, so nothing is tempted to
/// fetch it. Clauses 5.3.2 and 5.3.3 of the profile explicitly contemplate a
/// `vct` that is not a URL.
///
/// # Why the schema version is in it
///
/// Clause 2.2.2.1: *"The `vct` value also effectively identifies the version of
/// the credential type definition."* The passport's stored schema version is
/// already what selects the disclosure classes a signature was produced under,
/// so putting it here makes the credential name the exact ruleset it was issued
/// against — rather than a ruleset that may since have moved.
pub fn vct_for(product_group: ProductGroup, schema_version: &str) -> String {
    format!(
        "tag:{TAG_AUTHORITY}:vct:{}:{schema_version}",
        product_group.catalog_key()
    )
}
