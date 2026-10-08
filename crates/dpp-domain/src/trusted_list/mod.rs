//! EU Trusted List vocabulary — who is qualified, for what, and since when.
//!
//! ETSI TS 119 612, the standard Commission Implementing Regulations (EU)
//! 2025/1945 (validation) and 2025/1946 (preservation) both name normatively for
//! qualified signatures and seals. They name **V2.3.1**. **V2.4.1**, the current
//! publication, is the one that the trusted-list template of Implementing
//! Decision (EU) 2015/1505 names, as amended by Implementing Decision (EU)
//! 2025/2164.
//!
//! **For what this module reads, a citation of either is a citation of both.**
//! The two versions are the same in the service type URIs of clause 5.5.1, the
//! status values of clause 5.5.4 and Annex D.5, the status history of clauses
//! 5.5.5 and 5.6, and the retention rule of clauses 5.3.12 and 5.4.6. Compared
//! word for word, what changed between them is wording, cross-references and the
//! schema attachment, none of it in a clause this module reads.
//!
//! ✅ COMPLIANCE-PIN: Reg. (EU) No 910/2014, Art. 51(3), read in the consolidated
//! text of 18.10.2024 on 2026-10-06. Implementing Regulations (EU) 2025/1945 and
//! 2025/1946, and Implementing Decision (EU) 2025/2164, read in the Official
//! Journal text on 2026-10-06: the first two each list TS 119 612 V2.3.1 among
//! their referenced standards, and the third moves the template to V2.4.1.
//!
//! # Why this is in the domain and not in an adapter
//!
//! Fetching a trusted list is network work and belongs outside. *What the list
//! means* is not: the service type URIs, the status values, and the rule that
//! qualified status is a matter of a particular moment are all fixed by the
//! Regulation and the standard, and they change when those change. That is the
//! test this crate applies to everything it holds.
//!
//! # What this module is for
//!
//! Regulation (EU) No 910/2014 **Art. 22** requires Member States to publish
//! trusted lists "in a form suitable for automated processing", and the
//! Commission to publish the list of those lists. Art. 22 is what makes
//! qualified status a fact this software can establish rather than a claim it
//! has to accept from a vendor.
//!
//! Two questions need it:
//!
//! 1. **Was a seal's certificate qualified when the seal was made?**
//!    Art. 40 applies Art. 32 to seals, and Art. 32(1)(a)–(b) turns on the
//!    certificate having been a qualified certificate from a qualified provider
//!    *at the time of sealing*. Answering it is the difference between
//!    [`SealChecks::AdesValidation`](crate::seal::SealChecks::AdesValidation)
//!    and [`SealChecks::QualifiedValidation`](crate::seal::SealChecks::QualifiedValidation) —
//!    and the latter is currently unreachable because nothing consults a list.
//!
//! 2. **Is a cloud-sealing provider qualified for the service it is actually
//!    performing?** eIDAS 2 made the management of remote qualified seal
//!    creation devices its own trust service (**Art. 39a**), and the Art. 51(3)
//!    transitional that allowed it to be done without qualified status expired
//!    on **21 May 2026**. A provider holding the certificate leg and not that one
//!    cannot supply the creation-device limb of Art. 3(27).
//!    [`TrustServiceType::REMOTE_QSEAL_CD_MANAGEMENT`] is the exact entry to look
//!    for.
//!
//! # What this module is not
//!
//! It parses nothing, fetches nothing, and verifies no signature. It is
//! vocabulary and one rule about time. Retrieving lists, checking that each is
//! signed by its scheme operator, and matching a certificate to a service entry
//! are adapter work, and none of it is done yet — so no verdict in this crate
//! currently rests on a trusted list. Saying that plainly is the point: the
//! pieces that exist should not imply the pieces that do not.

mod history;
mod service_type;
mod status;
#[cfg(test)]
mod tests;

pub use history::{TrustServiceHistory, TrustServiceStatusPeriod};
pub use service_type::TrustServiceType;
pub use status::{GRANTED_URI, TrustServiceStatus, WITHDRAWN_URI};
