//! Disclosure rules — Commission Implementing Regulation (EU) 2026/2.
//!
//! # Two scopes, and they are not the same
//!
//! ESPR **Art. 25** prohibits the *destruction* of the unsold consumer products
//! in **Annex VII** — apparel, clothing accessories and footwear, and nothing
//! else. That scope lives in [`super::annex_vii`].
//!
//! ESPR **Art. 24** imposes a *disclosure* duty on discarded unsold **consumer
//! products** generally, and Impl. Reg. (EU) 2026/2 implements it. Its own
//! Annex II — the list this module carries — runs to 45 CN headings covering
//! soap, tyres, luggage, bed linen, air conditioners, refrigerators, computers,
//! batteries, lamps, furniture, toys and sanitary articles.
//!
//! **The disclosure is therefore much wider than the destruction ban.** Treating
//! Annex VII as the scope of the disclosure would silently drop every category
//! outside apparel and footwear from a report that is required to carry them.

use alloc::vec::Vec;

/// The CN headings of Annex II to Impl. Reg. (EU) 2026/2 — the consumer products
/// a disclosure must delimit at **four** digits rather than two.
///
/// Read from the OJ text (OJ L, 10.2.2026). Annex II's own preamble narrows it:
/// "Products listed in this Annex that are **components, intermediate products
/// or products that are not primarily intended for consumers** are not covered
/// by the obligation" — a limit on the *goods*, not on the code, and one no
/// table of headings can express. So membership here answers "which depth", not
/// "is this in scope".
const ANNEX_II_HEADINGS: &[&str] = &[
    "3401", "3402", "4011", "4202", "4203", "4303", "4818", "6301", "6302", "6303", "6304", "6306",
    "6307", "8415", "8418", "8421", "8422", "8423", "8443", "8450", "8467", "8471", "8506", "8507",
    "8508", "8509", "8510", "8513", "8516", "8517", "8518", "8519", "8521", "8523", "8524", "8527",
    "8528", "8539", "9006", "9401", "9403", "9404", "9503", "9504", "9619",
];

/// Whether a CN heading is listed in Annex II, and so must be disclosed at
/// four-digit depth.
#[must_use]
pub fn is_annex_ii_heading(heading: &str) -> bool {
    ANNEX_II_HEADINGS.contains(&heading)
}

/// Whether a disclosure line's CN category is filed at the depth **Art. 3**
/// requires.
///
/// Art. 3: categories are delimited on the **first two digits** of the CN code,
/// "however, the products listed in Annex II … shall be delimited based on the
/// **first four digits**".
///
/// So the test is asymmetric, and deliberately permissive in one direction:
///
/// - A 4-digit heading is always acceptable — it is required for Annex II
///   products and is strictly more precise than the two-digit default for
///   everything else.
/// - A 2-digit chapter is acceptable **unless** the chapter contains an Annex II
///   heading, in which case a product from it may have needed four digits and
///   the chapter has hidden which.
///
/// The second case cannot be decided from the code alone — a chapter holding an
/// Annex II heading also holds others — so this returns `false` and lets the
/// caller report it as a finding rather than an error.
#[must_use]
pub fn cn_depth_is_correct(cn_category: &str) -> bool {
    match cn_category.len() {
        4 => true,
        2 => !chapter_contains_annex_ii_heading(cn_category),
        _ => false,
    }
}

/// Whether any Annex II heading sits inside this CN chapter.
#[must_use]
pub fn chapter_contains_annex_ii_heading(chapter: &str) -> bool {
    ANNEX_II_HEADINGS.iter().any(|h| h.starts_with(chapter))
}

/// Every Annex II heading inside a chapter, so a finding can say which four-digit
/// codes the disclosure may have needed.
#[must_use]
pub fn annex_ii_headings_in_chapter(chapter: &str) -> Vec<&'static str> {
    ANNEX_II_HEADINGS
        .iter()
        .filter(|h| h.starts_with(chapter))
        .copied()
        .collect()
}

/// The share of a line that counts as **destroyed**.
///
/// Annex I note (i): "Destruction is the sum of recycling, other recovery and
/// disposal." Preparing for reuse and unknown are outside it.
///
/// Widened to `u16` because three `u8` shares can sum past 255 in a malformed
/// record, and a wrap would report a small number for a large problem.
#[must_use]
pub fn total_destruction_pct(recycling: u8, other_recovery: u8, disposal: u8) -> u16 {
    u16::from(recycling) + u16::from(other_recovery) + u16::from(disposal)
}

/// Whether a treatment split accounts for the whole line.
///
/// Note (i) has the percentages "calculated on the basis of the weight of
/// discarded unsold consumer products", and provides `unknown` for the share
/// whose treatment could not be established — so there is no share left over and
/// a well-formed split totals exactly 100.
#[must_use]
pub fn treatment_split_is_complete(
    preparing_for_reuse: u8,
    recycling: u8,
    other_recovery: u8,
    disposal: u8,
    unknown: u8,
) -> bool {
    u16::from(preparing_for_reuse)
        + u16::from(recycling)
        + u16::from(other_recovery)
        + u16::from(disposal)
        + u16::from(unknown)
        == 100
}

/// Whether a set of reasons used across one product category is admissible under
/// Del. Reg. (EU) 2026/296 Art. 2, point (h).
///
/// Point (h) — offered for donation and not accepted — applies "**only where
/// none of the circumstances referred to in points (a) to (g) are applicable**".
/// It is the one derogation defined by the absence of the others, so it cannot
/// be checked on a single line: the question is whether the operator claimed it
/// for a category it also claimed a stronger reason for.
///
/// `points` are the Art. 2 point letters used for one CN category in one
/// disclosure. Returns `false` where (h) appears alongside any of (a)–(g).
#[must_use]
pub fn donation_reason_is_admissible(points: &[char]) -> bool {
    let uses_h = points.contains(&'h');
    let uses_a_to_g = points.iter().any(|p| ('a'..='g').contains(p));
    !(uses_h && uses_a_to_g)
}
