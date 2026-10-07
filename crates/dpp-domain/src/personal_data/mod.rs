//! Personal data and the passport: what the operator states about it, and why
//! the data itself is never part of the passport.
//!
//! # The law
//!
//! Three acts that require a passport say the same thing about its content.
//! Regulation (EU) 2024/1781 (ESPR) Art. 10(1)(e): *"personal data relating to
//! customers shall not be stored in the digital product passport without their
//! explicit consent in compliance with Article 6 of Regulation (EU) 2016/679"*.
//! Regulation (EU) 2025/2509 (toys) Art. 20(10) and Regulation (EU) 2026/405
//! (detergents) Art. 22(h) carry the same sentence for their own passports. ESPR
//! Art. 2(35) defines a customer as anyone who buys, hires or receives a product
//! for their own use. Which acts say this is recorded in each act's manifest, as
//! `customerPersonalData`, and not here.
//!
//! Regulation (EU) 2023/1542 (batteries) has no such provision. Its passport
//! answers to Regulation (EU) 2016/679 (GDPR) alone, and to Art. 78(h), under
//! which the passport must ensure *"a high level of security and privacy"*. That
//! matters because the battery passport is the one that carries data about a
//! single unit in use: Annex XIII point 4 requires the record of each battery's
//! use, including *"negative events, such as accidents"*, and Art. 77(2) names
//! the battery's purchaser among those it may be shown to.
//!
//! # Why the personal data is never inside the passport
//!
//! Consent can be withdrawn at any time (GDPR Art. 7(3)). Once it is, and no
//! other ground applies, the controller must erase the data (Art. 17(1)(b)). A
//! published passport cannot be erased from: it is signed, its content is frozen
//! at publish, every version is archived, a copy sits with an independent back-up
//! provider (ESPR Art. 10(4)), and every reader who fetched it keeps a signed copy
//! that nobody can rewrite. Consent-based data placed in it could never be erased,
//! so it is kept out.
//!
//! The same holds for any personal data the passport is not required to carry,
//! whatever its basis. Data minimisation (GDPR Art. 5(1)(c)) limits it to what is
//! necessary, and data protection by default (Art. 25(2)) bars making it
//! accessible to an indefinite number of people without the person's own
//! intervention, which is what a public passport does.
//!
//! What a passport is *required* to carry is a different case. A battery's usage
//! record relates to the person who owns the battery whenever that person can be
//! identified, and GDPR Recital 26 counts the means reasonably likely to be used
//! by the controller *or by another person*. That record is still the passport's:
//! the Regulation requires it,
//! which is a legal obligation laid down by Union law (GDPR Art. 6(1)(c) and
//! 6(3)), and erasure does not reach data processed to comply with one (Art.
//! 17(3)(b)). Nothing in this module refuses it.
//!
//! # Marks, statements, and the refusal
//!
//! A product-group schema marks a property `"x-personal-data": true` when its
//! content is operator-written free text describing one item's life after sale.
//! That is where data about an owner, a user or someone else involved ends up.
//! Numbers and closed vocabularies are not marked, because they cannot hold more
//! than they are for. Neither is content an act requires, such as the contact
//! details of spare-part sources in battery Annex XIII point 2(b): storing it is
//! the obligation, and a refusal could not be right.
//!
//! A marked field with a value is refused at write time unless the passport's
//! `personalData` map names it with a [`PersonalDataStatement`]. The check is
//! `validate_passport` in the validation module, and it never reads what the
//! field says. Every statement asserts the same thing about the field's value:
//! **it carries no personal data beyond what the governing act requires the
//! passport itself to carry.** It then says where related personal data is held:
//! nowhere, or outside the passport, in an erasable record held under a named
//! [`LawfulBasis`].
//!
//! Where a governing act admits customer personal data only with explicit
//! consent, an `outside` statement must name [`LawfulBasis::Consent`].
//!
//! # What a statement cannot say
//!
//! That the basis is valid, or that consent was *"freely given, specific,
//! informed and unambiguous"* (GDPR Art. 4(11)). Those are the controller's to
//! establish and to demonstrate (Art. 5(2), Art. 7(1)), and nothing here can
//! check them. A false statement is possible. It is then the controller's own
//! false statement, signed with its own key, which is the accountability the
//! Regulation assumes.
//!
//! # Who signs it, and who sees it
//!
//! The statement is a member of the passport, so the operator's passport
//! signature covers it. Nothing new is signed, and nothing is signed by anyone
//! but the controller.
//!
//! A statement is shown only to an audience that can see the field it is about,
//! and never to the public. Even when it holds no personal data, it says whether
//! personal data related to one item exists, and that fact belongs to the
//! item's tier.
//!
//! # Where the held data lives
//!
//! In the record a [`PersonalDataRecordId`] names, kept through the
//! `PersonalDataPort` in the ports module. That record can be erased at any
//! time without touching the passport. Once it is erased, the identifier in the
//! signed passport points at nothing that relates to anyone. No passport view
//! ever includes the record: showing it to a reader is a disclosure that needs a
//! basis of its own, and only the controller can establish one.

mod held_outside;
mod lawful_basis;
mod record_id;
mod statement;
#[cfg(test)]
mod tests;

pub use held_outside::HeldOutside;
pub use lawful_basis::LawfulBasis;
pub use record_id::PersonalDataRecordId;
pub use statement::PersonalDataStatement;
