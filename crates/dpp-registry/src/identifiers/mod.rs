//! The persistent identifiers a DPP carries: product, product item, facility,
//! and economic operator.
//!
//! **Where these come from.** ESPR (EU) 2024/1781 **Annex III** ("Digital
//! product Passport", referred to in Articles 9 to 12) is what specifies the
//! data elements, including the unique product identifier (point (b)), the
//! GTIN (point (c)), unique operator identifiers (points **(g), (h) and (k)**)
//! and unique facility identifiers (point (i)). **Art. 12** imposes standards
//! conformity on the operator and facility identifiers; **Art. 13** sets up the
//! registry that *stores* them. Annex III names three identifier *types* —
//! product, operator, facility — so "the four Article 13 identifiers" was wrong
//! on both the count and the article, and is corrected here.
//!
//! **Point (k) belongs in that enumeration and was missing.** Annex III's second
//! paragraph names it explicitly alongside (g) and (h): "the unique operator
//! identifiers referred to in points (g), (h) and (k) … shall, where relevant
//! for the products concerned, comply with standards ISO/IEC 15459-1:2014 …".
//! Point (k) is the EU-established operator answerable for the Art. 4 of
//! Regulation (EU) 2019/1020 or Art. 16 of Regulation (EU) 2023/988 tasks, so
//! there are **three** classes of operator identifier, not two.
//!
//! That has a consequence for the shape: (g) and (h) travel as
//! [`OperatorIdentifier`], while point (k)'s identifier was a bare `String` on
//! `dpp_domain::operator::ResponsibleOperator`, and Annex III subjects all three
//! to the same conformity rule. All three are now readable as an EN 18219
//! clause 6 identifier, [`dpp_domain::ActorIdentifier`], and `ResponsibleOperator`
//! and `FacilitySnapshot` carry one in an **additive** optional field beside their
//! legacy strings, since the envelope has no lens and a signed passport cannot be
//! rewritten. This module's types keep their wire strings, because the registry's
//! own wire format is not one this crate can change on a guess. A registration
//! requires a clause 6 route of both identifiers: see
//! [`OperatorIdentifier::require_clause_6`].
//!
//! [`ProductItemIdentifier`] is not a fourth Annex III class: it is the product
//! identifier at item granularity, which Annex III point (b) leaves to "the
//! level indicated in the applicable delegated act". It is modelled separately
//! because serialised products need a distinct shape (batch/lot alongside the
//! serial), not because the Regulation lists it apart.
//!
//! [`ServiceProviderReference`] is Annex III **point (l)**, and is here for the
//! same reason: it names a party, with the same scheme/value/metadata shape.
//! It is deliberately *not* called an identifier — see the type.
//!
//! Grouped in one file rather than split per-type: they are one vocabulary
//! with symmetric shape (scheme + value + metadata), and share the
//! country-code / checksum validation helpers below.

mod registry_identifier;

pub use registry_identifier::*;
