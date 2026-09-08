//! SVHC concentration validation — REACH Art. 33 / ECHA SCIP.
//!
//! Cross-product group: applies to textiles, electronics, toys, construction, and more.
//! Kept here rather than under any single product group for that reason.

use alloc::{format, string::String, vec::Vec};

use super::cas::validate_cas_format;
use crate::common::numeric::percentage_in_range;

/// REACH Art. 33 threshold: at or above this w/w concentration in a finished article,
/// the supplier must proactively communicate SVHC presence to downstream recipients.
pub const SVHC_THRESHOLD_PCT: f64 = 0.1;

/// Embedded **partial** snapshot of the ECHA SVHC Candidate List (CAS numbers only).
///
/// Source: ECHA Candidate List of Substances of Very High Concern for Authorisation —
/// <https://echa.europa.eu/candidate-list-table>
///
/// ⚠️ **This is a small subset, not the list.** It holds well-established entries
/// across phthalates, bisphenols, chromium(VI) compounds, cobalt and key
/// solvents/monomers — see [`CandidateList::is_complete`] for how far short of the
/// official list that falls. A CAS absent from here yields
/// [`SvhcFindingKind::NotOnCheckedList`], which means *unknown to this snapshot*,
/// never *not an SVHC*.
///
/// COMPLIANCE-PIN: update on each ECHA Candidate List publication. Additions do
/// **not** follow a fixed calendar — the 2026-02-04 revision is the most recent —
/// so this cannot be maintained on a schedule; it has to track ECHA. Each addition
/// starts a six-month SCIP notification deadline for affected articles. Newly
/// removed SVHCs are extremely rare.
pub const ECHA_CANDIDATE_LIST: &[&str] = &[
    // ── Phthalates (reproductive toxicants) ──────────────────────────────────
    "117-81-7", // Bis(2-ethylhexyl) phthalate (DEHP)
    "84-74-2",  // Dibutyl phthalate (DBP)
    "85-68-7",  // Benzyl butyl phthalate (BBP)
    "84-69-5",  // Diisobutyl phthalate (DIBP)
    // ── Bisphenols (endocrine disruptors) ────────────────────────────────────
    "80-05-7",  // Bisphenol A (BPA)
    "80-09-1",  // Bisphenol S (BPS)
    "620-92-8", // Bisphenol F (BPF)
    // ── Chromium(VI) compounds (carcinogens / reproductive toxicants) ────────
    "1333-82-0",  // Chromium trioxide
    "10588-01-9", // Sodium dichromate
    "7778-50-9",  // Potassium dichromate
    "7789-09-5",  // Ammonium dichromate
    "7789-06-2",  // Strontium chromate
    "7758-97-6",  // Lead chromate
    "18454-12-1", // Lead sulfochromate yellow (C.I. Pigment Yellow 34)
    "13530-65-9", // Zinc chromate
    // ── Cobalt compounds (carcinogens) ───────────────────────────────────────
    "7646-79-9", // Cobalt dichloride
    "7440-48-4", // Cobalt (metal)
    // ── Heavy metals (reproductive toxicants / carcinogens) ──────────────────
    "7439-92-1", // Lead
    "7440-43-9", // Cadmium
    // ── Antimony (carcinogen) ─────────────────────────────────────────────────
    "1309-64-4", // Antimony trioxide
    // ── Polycyclic aromatic hydrocarbons — PBT / carcinogens ─────────────────
    "120-12-7", // Anthracene
    "91-20-3",  // Naphthalene
    // ── Chlorinated solvents (carcinogens) ───────────────────────────────────
    "79-01-6",  // Trichloroethylene
    "127-18-4", // Tetrachloroethylene (PERC)
    // ── Amides / formamides (reproductive toxicants) ─────────────────────────
    "75-12-7",  // Formamide
    "68-12-2",  // N,N-Dimethylformamide (DMF)
    "127-19-5", // N,N-Dimethylacetamide (DMAC)
    "872-50-4", // N-Methyl-2-pyrrolidone (NMP)
    // ── Other ────────────────────────────────────────────────────────────────
    "79-06-1",   // Acrylamide (carcinogen + mutagenic)
    "1763-23-1", // Perfluorooctane sulfonic acid (PFOS) — PBT
    // ── Added by the ECHA revision of 2026-02-04 ─────────────────────────────
    "110-54-3",  // n-Hexane (reproductive toxicant)
    "1478-61-1", // BPAF — 4,4'-(hexafluoroisopropylidene)diphenol, and its salts
];

/// Publication date of the ECHA revision this snapshot is aligned to.
pub const ECHA_CANDIDATE_LIST_AS_OF: &str = "2026-02-04";

/// Number of substances on the **official** Candidate List at
/// [`ECHA_CANDIDATE_LIST_AS_OF`].
pub const ECHA_CANDIDATE_LIST_OFFICIAL_COUNT: usize = 253;

/// A candidate list to check declarations against, carrying the provenance that
/// says which list it is.
///
/// # Why the data and its provenance are one type
///
/// They were two: a `const` slice and a separate `CandidateListProvenance`
/// describing it. That is safe only while there is exactly one list, because the
/// description cannot then be about anything else. The moment a newer list can
/// be supplied — which is the whole point of having a delivery channel — a
/// provenance value that travels separately can describe a *different* list than
/// the one a finding set was actually produced from, and a finding set is only
/// meaningful with respect to a specific list. Binding them removes that gap by
/// construction rather than by discipline.
///
/// A borrowed slice, so the crate stays `no_std` + `alloc` and a list can come
/// from anywhere the caller can borrow it from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateList<'a> {
    /// CAS numbers on this list.
    pub cas_numbers: &'a [&'a str],
    /// ECHA revision this list targets, ISO-8601.
    pub as_of: &'a str,
    /// Substances on the **official** list at `as_of`.
    ///
    /// Carried separately from `cas_numbers.len()` because the two differ: a
    /// partial list knows how far short it falls only if it is told.
    pub official_count: usize,
}

impl CandidateList<'static> {
    /// The compiled-in baseline: [`ECHA_CANDIDATE_LIST`] at
    /// [`ECHA_CANDIDATE_LIST_AS_OF`].
    ///
    /// What every check runs against when the caller supplies nothing.
    #[must_use]
    pub const fn embedded() -> Self {
        Self {
            cas_numbers: ECHA_CANDIDATE_LIST,
            as_of: ECHA_CANDIDATE_LIST_AS_OF,
            official_count: ECHA_CANDIDATE_LIST_OFFICIAL_COUNT,
        }
    }
}

impl CandidateList<'_> {
    /// Substances on this list.
    #[must_use]
    pub const fn entry_count(&self) -> usize {
        self.cas_numbers.len()
    }

    /// Whether this list covers the whole official list at its `as_of`.
    ///
    /// `false` for the embedded baseline, which holds 32 of 253. When it is
    /// `false`, a clean finding set means "nothing matched the substances this
    /// list knows about" — not "no SVHCs present".
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.entry_count() >= self.official_count
    }

    /// Whether `cas_number` appears on this list.
    #[must_use]
    pub fn contains(&self, cas_number: &str) -> bool {
        self.cas_numbers.contains(&cas_number)
    }
}

/// Classification of an SVHC declaration relative to the ECHA candidate list
/// and the REACH Art. 33 concentration threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvhcFindingKind {
    /// CAS is on the checked [`CandidateList`] and concentration ≥ [`SVHC_THRESHOLD_PCT`].
    /// Proactive supplier communication obligation is active under REACH Art. 33.
    MustDeclare,
    /// CAS is on the checked [`CandidateList`] but concentration < [`SVHC_THRESHOLD_PCT`].
    /// No Art. 33 obligation; entry is recorded for transparency.
    BelowThreshold,
    /// CAS is not present on the checked [`CandidateList`].
    ///
    /// The substance may still be an SVHC — the list may be partial (the
    /// compiled-in baseline is) or older than the substance's addition — or the
    /// CAS may be misidentified. Warrants manual review, and never reads as a
    /// clearance.
    ///
    /// Named `NotOnCheckedList` until the list became a parameter, when the
    /// only list there was was the embedded one. The old name would now be a
    /// lie in the case that matters most: a bundle-supplied list is not the
    /// embedded one, and a reader told a substance is "not in the embedded
    /// list" would draw a conclusion about the wrong list.
    NotOnCheckedList,
}

/// Semantic finding for one entry in an SVHC declaration.
#[derive(Debug, Clone, Copy)]
pub struct SvhcFinding<'a> {
    pub index: usize,
    pub cas_number: &'a str,
    pub substance_name: &'a str,
    pub concentration_pct: f64,
    pub kind: SvhcFindingKind,
}

/// A substance-of-very-high-concern entry for validation.
#[derive(Debug, Clone, Copy)]
pub struct SvhcInput<'a> {
    pub cas_number: &'a str,
    pub substance_name: &'a str,
    pub concentration_pct: f64,
}

/// Structural validation: non-empty CAS in valid format, non-empty name, finite
/// concentration in [0, 100].
///
/// An empty list is valid — it means the manufacturer checked and found no SVHCs.
pub fn validate_svhc_substances(substances: &[SvhcInput<'_>]) -> Result<(), String> {
    for (i, s) in substances.iter().enumerate() {
        if s.cas_number.is_empty() {
            return Err(format!(
                "svhc_substances[{i}]: cas_number must not be empty"
            ));
        }
        if let Err(e) = validate_cas_format(s.cas_number) {
            return Err(format!("svhc_substances[{i}]: invalid CAS number: {e}"));
        }
        if s.substance_name.is_empty() {
            return Err(format!(
                "svhc_substances[{i}]: substance_name must not be empty"
            ));
        }
        if !percentage_in_range(s.concentration_pct) {
            return Err(format!(
                "svhc_substances[{i}]: concentration_pct must be a finite value in 0–100, got {}",
                s.concentration_pct
            ));
        }
    }
    Ok(())
}

/// Semantic analysis: classify each declaration against `list` and the REACH
/// Art. 33 threshold.
///
/// Returns one [`SvhcFinding`] per input entry. Always call
/// [`validate_svhc_substances`] first — this function skips structural checks and
/// treats its inputs as well-formed.
///
/// # Why the list is a parameter
///
/// ECHA adds to the Candidate List on no fixed calendar, and each addition
/// starts a six-month SCIP notification deadline. With the list compiled in, the
/// only way to deliver a newer one is a crate release, and an operator who skips
/// the upgrade keeps checking against a stale list with no signal that they are
/// behind. Taking it as a parameter lets a caller supply a list that arrived
/// through some other channel; it does not itself provide that channel.
///
/// Pass [`CandidateList::embedded()`] for the compiled-in baseline —
/// [`check_svhc_declarations_embedded`] is that call, named.
pub fn check_svhc_declarations<'a>(
    substances: &[SvhcInput<'a>],
    list: &CandidateList<'_>,
) -> Vec<SvhcFinding<'a>> {
    substances
        .iter()
        .enumerate()
        .map(|(index, s)| {
            let kind = if !list.contains(s.cas_number) {
                SvhcFindingKind::NotOnCheckedList
            } else if s.concentration_pct >= SVHC_THRESHOLD_PCT {
                SvhcFindingKind::MustDeclare
            } else {
                SvhcFindingKind::BelowThreshold
            };
            SvhcFinding {
                index,
                cas_number: s.cas_number,
                substance_name: s.substance_name,
                concentration_pct: s.concentration_pct,
                kind,
            }
        })
        .collect()
}

/// [`check_svhc_declarations`] against the compiled-in baseline.
///
/// The behaviour of a build that has been given no other list — which is every
/// build today.
#[must_use]
pub fn check_svhc_declarations_embedded<'a>(substances: &[SvhcInput<'a>]) -> Vec<SvhcFinding<'a>> {
    check_svhc_declarations(substances, &CandidateList::embedded())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bpa(pct: f64) -> SvhcInput<'static> {
        SvhcInput {
            cas_number: "80-05-7",
            substance_name: "Bisphenol A",
            concentration_pct: pct,
        }
    }

    // ── Structural validation ────────────────────────────────────────────────

    #[test]
    fn svhc_valid_and_invalid() {
        assert!(validate_svhc_substances(&[bpa(0.15)]).is_ok());
        assert!(validate_svhc_substances(&[]).is_ok()); // empty = checked, none found

        let empty_cas = SvhcInput {
            cas_number: "",
            substance_name: "x",
            concentration_pct: 0.5,
        };
        assert!(validate_svhc_substances(&[empty_cas]).is_err());

        let bad_conc = SvhcInput {
            cas_number: "80-05-7",
            substance_name: "x",
            concentration_pct: -1.0,
        };
        assert!(validate_svhc_substances(&[bad_conc]).is_err());
    }

    #[test]
    fn nan_concentration_rejected() {
        assert!(validate_svhc_substances(&[bpa(f64::NAN)]).is_err());
    }

    #[test]
    fn infinity_concentration_rejected() {
        assert!(validate_svhc_substances(&[bpa(f64::INFINITY)]).is_err());
    }

    #[test]
    fn malformed_cas_rejected_by_structural_validator() {
        // 80-05-8 has a wrong check digit (correct is 7).
        let bad = SvhcInput {
            cas_number: "80-05-8",
            substance_name: "BPA wrong CAS",
            concentration_pct: 0.5,
        };
        let err = validate_svhc_substances(&[bad]).unwrap_err();
        assert!(err.contains("check digit"), "unexpected: {err}");
    }

    #[test]
    fn wrong_cas_format_rejected() {
        let bad = SvhcInput {
            cas_number: "NOTACAS",
            substance_name: "X",
            concentration_pct: 0.1,
        };
        assert!(validate_svhc_substances(&[bad]).is_err());
    }

    // ── Candidate list + threshold semantics ─────────────────────────────────

    #[test]
    fn bpa_above_threshold_is_must_declare() {
        let findings = check_svhc_declarations_embedded(&[bpa(0.15)]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, SvhcFindingKind::MustDeclare);
    }

    #[test]
    fn bpa_below_threshold_is_informational() {
        let findings = check_svhc_declarations_embedded(&[bpa(0.05)]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, SvhcFindingKind::BelowThreshold);
    }

    #[test]
    fn bpa_exactly_at_threshold_is_must_declare() {
        // Boundary: >= 0.1 triggers obligation.
        let findings = check_svhc_declarations_embedded(&[bpa(SVHC_THRESHOLD_PCT)]);
        assert_eq!(findings[0].kind, SvhcFindingKind::MustDeclare);
    }

    #[test]
    fn unknown_cas_flagged_as_not_in_embedded_list() {
        // 7440-66-6 = Zinc — valid CAS format, not an SVHC.
        let zinc = SvhcInput {
            cas_number: "7440-66-6",
            substance_name: "Zinc",
            concentration_pct: 5.0,
        };
        let findings = check_svhc_declarations_embedded(&[zinc]);
        assert_eq!(findings[0].kind, SvhcFindingKind::NotOnCheckedList);
    }

    #[test]
    fn mixed_declarations_produce_correct_findings() {
        let above = bpa(0.2);
        let below = SvhcInput {
            cas_number: "84-74-2",
            substance_name: "DBP",
            concentration_pct: 0.05,
        };
        let unknown = SvhcInput {
            cas_number: "7440-66-6",
            substance_name: "Zinc",
            concentration_pct: 5.0,
        };
        let findings = check_svhc_declarations_embedded(&[above, below, unknown]);
        assert_eq!(findings[0].kind, SvhcFindingKind::MustDeclare);
        assert_eq!(findings[1].kind, SvhcFindingKind::BelowThreshold);
        assert_eq!(findings[2].kind, SvhcFindingKind::NotOnCheckedList);
        // Indices are preserved
        assert_eq!(findings[0].index, 0);
        assert_eq!(findings[1].index, 1);
        assert_eq!(findings[2].index, 2);
    }

    #[test]
    fn threshold_constant_is_0_1_pct() {
        assert!(
            (SVHC_THRESHOLD_PCT - 0.1).abs() < f64::EPSILON,
            "REACH Art. 33 threshold must be exactly 0.1 % w/w"
        );
    }

    #[test]
    fn bpa_on_candidate_list() {
        assert!(
            ECHA_CANDIDATE_LIST.contains(&"80-05-7"),
            "BPA (80-05-7) must be on the embedded candidate list"
        );
    }

    #[test]
    fn all_embedded_cas_numbers_have_valid_format() {
        // Verifies that no data-entry error was made when compiling the candidate list.
        for &cas in ECHA_CANDIDATE_LIST {
            let s = [SvhcInput {
                cas_number: cas,
                substance_name: "test",
                concentration_pct: 0.5,
            }];
            assert!(
                validate_svhc_substances(&s).is_ok(),
                "embedded candidate list entry fails CAS format check: {cas}"
            );
        }
    }
}

#[cfg(test)]
mod provenance_tests {
    use super::*;

    #[test]
    fn the_embedded_list_reports_its_own_size_and_revision() {
        let list = CandidateList::embedded();
        assert_eq!(list.entry_count(), ECHA_CANDIDATE_LIST.len());
        assert_eq!(list.as_of, "2026-02-04");
        assert_eq!(list.official_count, ECHA_CANDIDATE_LIST_OFFICIAL_COUNT);
    }

    #[test]
    fn snapshot_is_known_incomplete() {
        // The point of this type: a caller must be able to see that a clean
        // finding set was produced from a partial list. If this ever becomes
        // true, the coverage gap has genuinely been closed and the docs saying
        // otherwise need updating with it.
        assert!(
            !CandidateList::embedded().is_complete(),
            "snapshot now claims completeness — update the module docs and the \
             adjacent-obligations research note if that is genuinely true"
        );
    }

    #[test]
    fn a_supplied_list_is_checked_instead_of_the_embedded_one() {
        // The whole point of the parameter: a substance the embedded snapshot
        // has never heard of is recognised when a newer list carries it.
        const NEWER: &[&str] = &["7440-02-0"];
        let list = CandidateList {
            cas_numbers: NEWER,
            as_of: "2027-01-01",
            official_count: 1,
        };
        let nickel = SvhcInput {
            cas_number: "7440-02-0",
            substance_name: "Nickel",
            concentration_pct: 0.5,
        };

        assert_eq!(
            check_svhc_declarations_embedded(&[nickel])[0].kind,
            SvhcFindingKind::NotOnCheckedList,
            "the embedded snapshot does not carry this CAS"
        );
        assert_eq!(
            check_svhc_declarations(&[nickel], &list)[0].kind,
            SvhcFindingKind::MustDeclare,
            "the supplied list does, so the same declaration classifies differently"
        );
    }

    #[test]
    fn the_2026_02_04_additions_are_present() {
        assert!(
            ECHA_CANDIDATE_LIST.contains(&"110-54-3"),
            "n-Hexane missing"
        );
        assert!(ECHA_CANDIDATE_LIST.contains(&"1478-61-1"), "BPAF missing");
    }

    #[test]
    fn embedded_list_has_no_duplicates() {
        for (i, cas) in ECHA_CANDIDATE_LIST.iter().enumerate() {
            assert!(
                !ECHA_CANDIDATE_LIST[..i].contains(cas),
                "duplicate CAS {cas} in the embedded candidate list"
            );
        }
    }

    #[test]
    fn absent_cas_is_unknown_not_cleared() {
        // A real SVHC we do not embed must not read as "fine". This guards the
        // semantics of NotOnCheckedList against a future well-meaning rename.
        let unknown = SvhcInput {
            cas_number: "50-00-0", // formaldehyde — on the official list, not embedded
            substance_name: "Formaldehyde",
            concentration_pct: 5.0,
        };
        let f = check_svhc_declarations_embedded(&[unknown]);
        assert_eq!(f[0].kind, SvhcFindingKind::NotOnCheckedList);
    }
}
