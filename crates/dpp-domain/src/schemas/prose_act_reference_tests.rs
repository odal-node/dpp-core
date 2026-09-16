//! Reading an EU act reference out of prose, and deciding whether a description
//! cites an article or an annex at all.
//!
//! Split from `prose_citation_tests`, which holds the rules. These are the
//! detectors those rules are built on, with their own tests — a false positive
//! here would fail the gate on correct prose, which is how a gate gets disabled
//! rather than fixed.

// ── detectors ────────────────────────────────────────────────────────────────

/// An act reference found in prose, and the CELEX identifier it resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActRef {
    pub(super) celex: String,
    pub(super) text: String,
}

/// Whether `prose` cites an article or an annex.
///
/// `Annex` must be followed by roman numerals: `Annex XIII` is a citation,
/// "annexed to the report" is not. `Art.` and `Article` must be followed by a
/// digit, so "Article 33" counts and "the articles it covers" does not.
pub(super) fn cites_article_or_annex(prose: &str) -> bool {
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
pub(super) fn act_refs(prose: &str) -> Vec<ActRef> {
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

/// The gate is only worth having if it fails on the shapes it exists to catch.
///
/// Asserted directly on the detectors rather than by mutating a schema, so the
/// evidence lives beside the rules instead of in a commit message somebody has
/// to find.
#[test]
fn the_detectors_catch_what_they_are_for() {
    // Rule B — the original defect: an act number that resolves to nothing.
    let invented = act_refs("Per Regulation (EU) 2027/9999 Annex II.");
    assert_eq!(
        invented.first().map(|a| a.celex.as_str()),
        Some("32027R9999"),
        "an invented act number must still parse, or Rule B never sees it"
    );

    // Rule A — an unanchored annex citation.
    assert!(cites_article_or_annex(
        "SVHC substances per REACH Article 33."
    ));
    assert!(cites_article_or_annex(
        "Contact allergens under Annex XVII entry 72."
    ));
    assert!(!cites_article_or_annex(
        "Recycled content share as a percentage of total mass."
    ));

    // Both numbering conventions, and the two act kinds.
    for (prose, expected) in [
        ("Regulation (EU) 2023/1670", "32023R1670"),
        ("Regulation (EC) No 1907/2006", "32006R1907"),
        ("Directive 2011/65/EU", "32011L0065"),
        ("Directive (EU) 2017/1132", "32017L1132"),
        ("EU Battery Regulation 2023/1542", "32023R1542"),
        ("replacing 1222/2009", "32009R1222"),
        // A sentence-ending act number. This was silently dropped while the
        // trailing-separator guard treated a full stop as a version separator,
        // which made a correctly anchored schema look unanchored.
        ("Toy fields per Regulation (EU) 2025/2509.", "32025R2509"),
    ] {
        assert_eq!(
            act_refs(prose).first().map(|a| a.celex.as_str()),
            Some(expected),
            "{prose} should resolve to {expected}"
        );
    }

    // Things that look like act numbers and are not. A false positive here would
    // make Rule B fail on correct prose, which is how a gate gets disabled.
    for prose in [
        "ISO/IEC 15459-1:2014, -2:2015 and -3:2014",
        "v1.1.0 renames countryOfManufacture",
        "above 0,1 % w/w",
        "placed on the market from 2031-08-18",
        "Annex VI Part A point 10",
        "ranked 1/2 in the working plan",
    ] {
        assert!(
            act_refs(prose).is_empty(),
            "{prose} must not be read as an act reference, got {:?}",
            act_refs(prose)
        );
    }
}

/// Whether an inventory entry's *reason* was read out of the primary text, or
/// written from something weaker.
///
/// The same distinction `ParameterBasis` draws for calculation inputs and
/// `RetentionBasis` and `DateBasis` draw elsewhere in this workspace, applied to
/// the claim a reason makes. Reusing the vocabulary rather than inventing a
/// marker convention is deliberate: a bespoke one would be understood only by
/// the rule that reads it.
///
/// # The default runs the other way here, and that is not an oversight
///
/// `ParameterBasis` defaults to `Sourced`, because treating law as ours silently
/// replaces a legal threshold while treating ours as law only leaves a visible
/// placeholder. **The asymmetry inverts for a citation reason.** Marking an
/// unverified reason `Sourced` asserts that someone read the Official Journal
/// when nobody did, and that assertion is invisible — the reasons read
/// identically either way, which is the whole defect this inventory was found to
/// have. So an entry is [`Assumed`](Self::Assumed) until a reader has been to the
/// text, and `Sourced` costs an article or annex number that
/// [`a_sourced_reason_cites_an_article_or_annex`] checks for.
///
/// [`a_sourced_reason_cites_an_article_or_annex`]: super::prose_citation_tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CitationBasis {
    /// Read against the Official Journal text of the act it describes. The
    /// reason names the article or annex it was read from.
    Sourced,
    /// Written from recall, a secondary source, or an adjacent act's account of
    /// this one. It may well be right; nobody has been to the text.
    Assumed,
}

/// One act cited in schema prose that the instrument catalog does not model.
pub(super) struct CitedNotModelled {
    /// CELEX identifier of the cited act.
    pub(super) celex: &'static str,
    /// Why prose legitimately cites an act no binding describes.
    pub(super) reason: &'static str,
    /// Whether that reason was read out of the primary text.
    pub(super) basis: CitationBasis,
}

/// Acts cited in schema prose that the instrument catalog does not model.
///
/// Not a suppression list — an inventory. An act appears here because prose
/// legitimately cites it while no binding in `crates/dpp-domain/instruments/`
/// describes it, which is a different statement from "we have not checked it".
/// Anything cited and *not* listed here must resolve to a catalog entry, so an
/// invented act number fails this test rather than shipping.
///
/// Adding an entry is a deliberate act with a reason attached. Removing one
/// happens when the instrument gets modelled.
///
/// # Why each entry carries a basis
///
/// **A reason is a claim.** The reasons are what make this inventory reviewable
/// rather than a list of exemptions, and several of them assert substantive
/// content about acts — a repeal date, an annex part, the absence of an
/// obligation across a whole regulation. Nothing used to distinguish a reason
/// verified against the Official Journal from one written from recall, and they
/// read identically. This repository has already had a citation inverted by
/// exactly that gap, when an act was described as current on the day it turned
/// out to have been repealed.
pub(super) const CITED_NOT_MODELLED: &[CitedNotModelled] = &[
    CitedNotModelled {
        celex: "32004R0648",
        reason: "The old Detergents Regulation. Cited only as the act Regulation \
                 (EU) 2026/405 repeals: its Art. 36 reads 'Regulation (EC) No \
                 648/2004 is repealed with effect from 23 September 2029.' The \
                 same article carries a grandfathering window to 23 September \
                 2030 for product placed on the market in the preceding year, so \
                 'repealed' on its own overstates how cleanly it ends — which is \
                 the transition a reader needs.",
        basis: CitationBasis::Sourced,
    },
    CitedNotModelled {
        celex: "32006R1907",
        reason: "REACH. Cited for Art. 33 (SVHC communication duty above 0,1 % \
                 w/w) and Annex XVII entry 72 (restricted substances in \
                 textiles). A horizontal chemicals regime rather than a passport \
                 instrument, so it binds no product group in the catalog sense.",
        basis: CitationBasis::Assumed,
    },
    CitedNotModelled {
        celex: "32009L0048",
        reason: "Toy Safety Directive. Cited for CE marking. Superseded for \
                 passport purposes by Regulation (EU) 2025/2509, which is \
                 modelled.",
        basis: CitationBasis::Assumed,
    },
    CitedNotModelled {
        celex: "32009R0661",
        reason: "General safety of motor vehicles. Cited only as the source of \
                 the tyre noise limit values (LV) that Regulation (EU) 2020/740 \
                 grades against — a threshold this crate reads, not an \
                 obligation it carries. 2020/740's own Annex I Part C says so: \
                 'The external rolling noise class shall be determined … on the \
                 basis of the limit values (LV) set out in Part C of Annex II to \
                 Regulation (EC) No 661/2009.'",
        basis: CitationBasis::Sourced,
    },
    CitedNotModelled {
        celex: "32009R1222",
        reason: "The old tyre labelling regulation. Cited only as the act \
                 Regulation (EU) 2020/740 replaced. Its Art. 17 reads \
                 'Regulation (EC) No 1222/2009 is repealed with effect from 1 \
                 May 2021', and directs that references to it be read against \
                 the correlation table in Annex VIII.",
        basis: CitationBasis::Sourced,
    },
    CitedNotModelled {
        celex: "32011L0065",
        reason: "RoHS. Cited for the substance restrictions an electronics \
                 declaration references. Not a passport instrument.",
        basis: CitationBasis::Assumed,
    },
    CitedNotModelled {
        celex: "32017L1132",
        reason: "Company law directive. Cited for Art. 16, which establishes the \
                 unique company identifier the unsold-goods schema uses for the \
                 EUID.",
        basis: CitationBasis::Assumed,
    },
    CitedNotModelled {
        celex: "32020R0740",
        reason: "Tyre labelling. Cited for the Annex I grading scales. A \
                 labelling regime, not a passport obligation — the tyre passport \
                 duty, when one exists, will come from an ESPR delegated act.",
        basis: CitationBasis::Assumed,
    },
    CitedNotModelled {
        celex: "32023R1669",
        reason: "Energy labelling for smartphones and slate tablets. The sibling \
                 of Regulation (EU) 2023/1670, which is modelled; this one sets \
                 label classes rather than passport content.",
        basis: CitationBasis::Assumed,
    },
    CitedNotModelled {
        celex: "32024R1252",
        reason: "Critical Raw Materials Act. Cited as the source of the \
                 canonical CRM list. The claim that it defines which materials \
                 are critical without governing their disclosure is the hardest \
                 kind to hold — it asserts the absence of an obligation across a \
                 whole regulation — and nobody holds the text. This entry is the \
                 one the inventory most needs read, because the claim also \
                 ships, in every battery and electronics schema description \
                 naming the act.",
        basis: CitationBasis::Assumed,
    },
];
