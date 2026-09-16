//! Reading an EU act reference out of prose, and deciding whether a description
//! cites an article or an annex at all.
//!
//! # Why this is public
//!
//! These detectors began as the private half of the schema-prose citation gate,
//! which walks JSON `description` strings. They are not specific to JSON: the
//! same question — *which act does this sentence cite, and does it name an
//! article or an annex of it* — is asked of Rust doc comments by the gate in the
//! cross-crate test tier, which cannot see a `#[cfg(test)]` module here.
//!
//! Sharing them rather than copying them is the point. A second implementation
//! would drift, and the drift would show up as one surface being checked to a
//! standard the other is not.
//!
//! They are deliberately conservative: a false positive fails the gate on
//! correct prose, which is how a gate gets disabled rather than fixed.

/// An act reference found in prose, and the CELEX identifier it resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActRef {
    /// CELEX identifier the reference resolves to, e.g. `32023R1542`.
    pub celex: String,
    /// The reference exactly as the prose spells it, for an error message that
    /// a reader can find in the file.
    pub text: String,
}

/// Whether `prose` cites an article or an annex.
///
/// `Annex` must be followed by roman numerals: `Annex XIII` is a citation,
/// "annexed to the report" is not. `Art.` and `Article` must be followed by a
/// digit, so "Article 33" counts and "the articles it covers" does not.
pub fn cites_article_or_annex(prose: &str) -> bool {
    let has_roman_annex = prose.match_indices("Annex ").any(|(i, _)| {
        prose[i + "Annex ".len()..]
            .chars()
            .next()
            .is_some_and(|c| "IVXLCDM".contains(c))
    });
    let has_article = ["Art. ", "Article "].iter().any(|marker| {
        prose.match_indices(marker).any(|(i, _)| {
            prose[i + marker.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        })
    });
    has_roman_annex || has_article
}

/// Every act reference in `prose`, resolved to CELEX.
///
/// Handles both EU numbering conventions, which run in opposite orders:
/// post-2015 acts are `year/number` (Regulation (EU) 2023/1670) and older ones
/// are `number/year` (Regulation (EC) No 1907/2006). Whichever side is a
/// plausible four-digit year is taken as the year — there is no act numbered
/// above 9999 in a year below 1950, so this is unambiguous in practice.
///
/// Directives are distinguished from regulations by the trailing `/EU` `/EC`
/// form (`2011/65/EU`), and otherwise by whichever of the words "directive" and
/// "regulation" sits closer in front of the number. That matters because
/// `Directive (EU) 2017/1132` and `Regulation (EU) 2017/1132` differ only in the
/// word, and they are different acts.
pub fn act_refs(prose: &str) -> Vec<ActRef> {
    let bytes = prose.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        // A digit run that continues an identifier — `15459-1`, `v1.1.0`,
        // `2031-08-18` — is not the start of an act number.
        if i > 0 && matches!(bytes[i - 1], b'-' | b'.' | b',') {
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            continue;
        }

        let first_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let first_len = i - first_start;
        let Ok(first) = prose[first_start..i].parse::<u32>() else {
            continue;
        };

        if i >= bytes.len() || bytes[i] != b'/' {
            continue;
        }
        let after_slash = i + 1;
        i = after_slash;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == after_slash {
            continue;
        }
        let second_len = i - after_slash;
        let Ok(second) = prose[after_slash..i].parse::<u32>() else {
            continue;
        };
        // `15459-1:2014` and `v1.1.0` style tails: a digit run continuing into
        // another number is a standard or version reference, not an act. The
        // separator must be followed by a *digit* to disqualify — a sentence
        // ending "Regulation (EU) 2025/2509." is an act reference, and reading
        // that full stop as a version separator silently dropped every act
        // number that ended a sentence.
        if i + 1 < bytes.len() && matches!(bytes[i], b'-' | b'.') && bytes[i + 1].is_ascii_digit() {
            continue;
        }

        let mut end = i;
        let mut directive_by_form = false;
        if i < bytes.len() && bytes[i] == b'/' {
            let suffix_start = i + 1;
            let mut j = suffix_start;
            while j < bytes.len() && bytes[j].is_ascii_uppercase() {
                j += 1;
            }
            if matches!(&prose[suffix_start..j], "EU" | "EC" | "EEC") {
                directive_by_form = true;
                end = j;
            }
        }

        let is_year = |y: u32| (1950..=2099).contains(&y);
        let (year, number) = if first_len == 4 && is_year(first) {
            (first, second)
        } else if second_len == 4 && is_year(second) {
            (second, first)
        } else {
            i = end;
            continue;
        };
        if number == 0 || number > 9999 {
            i = end;
            continue;
        }

        let kind = if directive_by_form || nearest_kind_word_is_directive(&prose[..first_start]) {
            'L'
        } else {
            'R'
        };

        found.push(ActRef {
            celex: format!("3{year}{kind}{number:04}"),
            text: prose[first_start..end].to_owned(),
        });
        i = end;
    }

    found
}

/// Whether "directive" sits closer than "regulation" in the text preceding an
/// act number, within a clause's reach.
fn nearest_kind_word_is_directive(before: &str) -> bool {
    let lowered = before.to_lowercase();
    let within_reach = |found: Option<usize>| {
        found.filter(|position| lowered.len().saturating_sub(*position) <= 60)
    };
    let directive = within_reach(lowered.rfind("directive"));
    let regulation = within_reach(lowered.rfind("regulation"));
    match (directive, regulation) {
        (Some(d), Some(r)) => d > r,
        (Some(_), None) => true,
        _ => false,
    }
}
