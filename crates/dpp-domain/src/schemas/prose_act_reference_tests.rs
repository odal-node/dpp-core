//! The `CITED_NOT_MODELLED` inventory, and the detectors' own tests.
//!
//! The detectors themselves now live in [`super::citation`], which is public
//! because the doc-comment citation gate in the cross-crate test tier needs them
//! and cannot see a `#[cfg(test)]` module. Their tests stay here, beside the
//! rules they were written for.

use super::citation::{act_refs, cites_article_or_annex};

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

    // Both numbering conventions, and every act kind the sector letter can be.
    for (prose, expected) in [
        ("Regulation (EU) 2023/1670", "32023R1670"),
        ("Regulation (EC) No 1907/2006", "32006R1907"),
        ("Directive 2011/65/EU", "32011L0065"),
        ("Directive (EU) 2017/1132", "32017L1132"),
        ("EU Battery Regulation 2023/1542", "32023R1542"),
        ("replacing 1222/2009", "32009R1222"),
        // 🚨 Decisions. There was no arm for them, so every one resolved as a
        // Regulation — a CELEX that points at a different act, or at nothing.
        // Both of these are cited in this workspace today.
        (
            "Commission Implementing Decision (EU) 2026/1736",
            "32026D1736",
        ),
        (
            "Commission Implementing Decision (EU) 2015/1506",
            "32015D1506",
        ),
        // The form the old rule got backwards: a Decision carrying the trailing
        // `/EU` that used to be read as proof of a Directive.
        ("Commission Decision 2011/833/EU", "32011D0833"),
        // Nothing cites a Recommendation yet. Pinned anyway, because the cost of
        // the missing arm is a wrong CELEX rather than a failure.
        ("Commission Recommendation (EU) 2021/2279", "32021H2279"),
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

    // Each act in a sentence takes the kind word nearest *it*, not the sentence's
    // first one — the shape a real citation has once more than one act is named.
    assert_eq!(
        act_refs(
            "Presumption under Regulation (EU) 2024/1781 Art. 41(2), via the \
             standards cited by Commission Implementing Decision (EU) 2026/1736."
        )
        .iter()
        .map(|a| a.celex.as_str())
        .collect::<Vec<_>>(),
        ["32024R1781", "32026D1736"],
    );

    // 🚨 A kind word must *start* a word. Matching inside one let "indecision"
    // name a Decision and "deregulation" name a Regulation, which does not fail
    // — it silently sets the sector letter from a word that is not an act type,
    // and the result is a well-formed CELEX for a different act.
    for (prose, expected) in [
        ("Market indecision since 2023/1234", "32023R1234"),
        (
            "After deregulation, Directive 2011/65/EU applied",
            "32011L0065",
        ),
        // Inflections are the same word and must still count.
        ("Both regulations, including 2023/1670", "32023R1670"),
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
                 (EU) 2026/405 repeals: its Art. 35 reads 'Regulation (EC) No \
                 648/2004 is repealed with effect from 23 September 2029', and \
                 directs that references be read against the correlation table in \
                 Annex VIII. The *next* article, Art. 36, carries the transition: \
                 product placed before 23 September 2029 may be made available \
                 indefinitely, and product placed in the following year until 23 \
                 September 2030 — so 'repealed' on its own overstates how cleanly \
                 it ends, which is what a reader needs. This entry said Art. 36 \
                 for both, while marked as read against the Official Journal.",
        basis: CitationBasis::Sourced,
    },
    CitedNotModelled {
        celex: "32006R1907",
        reason: "REACH. Cited for two provisions. Art. 33(1) makes the supplier of \
                 an article containing a substance meeting the Art. 57 criteria \
                 and identified under Art. 59(1) in a concentration above 0,1 % \
                 weight by weight give the recipient 'sufficient information, \
                 available to the supplier, to allow safe use of the article \
                 including, as a minimum, the name of that substance'; Art. 33(2) \
                 gives a consumer the same on request, within 45 days. Annex XVII \
                 entry 72 bars clothing, related accessories, skin-contact \
                 textiles and footwear for consumers from the market when a \
                 substance listed in Appendix 12 is present at or above its \
                 limit, measured in homogeneous material. Appendix 12 is a long \
                 list with a limit per substance: cadmium, chromium VI, arsenic \
                 and lead compounds, benzene, polycyclic aromatic hydrocarbons, \
                 formaldehyde, several phthalates, N-methyl-2-pyrrolidone, \
                 N,N-dimethylacetamide, N,N-dimethylformamide and a run of dyes \
                 and amines, in the consolidation of 22 June 2026. A horizontal \
                 chemicals regime rather than a passport instrument: the word \
                 'passport' does not occur in that text, so it binds no product \
                 group in the catalog sense.",
        basis: CitationBasis::Sourced,
    },
    CitedNotModelled {
        celex: "32009L0048",
        reason: "Toy Safety Directive. Cited by the toy schema as the regime under \
                 which a toy bears CE marking: Art. 16(1) says toys made \
                 available on the market shall bear it, and Art. 17 sets how and \
                 where it is affixed. Art. 15 is the EC declaration of \
                 conformity. It is not repealed yet: Regulation (EU) 2025/2509 \
                 Art. 56 reads 'Directive 2009/48/EC is repealed with effect \
                 from 1 August 2030'. Art. 57(1) lets toys placed on the market \
                 in conformity with the Directive before that date stay on it, \
                 and Art. 57(3) keeps EC type-examination certificates valid \
                 until 1 February 2031 unless they expire sooner. The passport \
                 comes from the Regulation, which is modelled, and its recital \
                 54 says the passport 'should replace the EU declaration of \
                 conformity pursuant to Directive 2009/48/EC'. The Directive is \
                 not a passport instrument: the word 'passport' does not occur \
                 in it as consolidated on 29 August 2026.",
        basis: CitationBasis::Sourced,
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
                 declaration references. Art. 4(1) requires Member States to \
                 ensure that electrical and electronic equipment placed on the \
                 market 'does not contain the substances listed in Annex II', \
                 and Annex II lists ten, each with a maximum concentration by \
                 weight in homogeneous materials: lead, mercury, cadmium, \
                 hexavalent chromium, PBB, PBDE and four phthalates (DEHP, BBP, \
                 DBP, DIBP). 🚨 The 2011 text lists six; the phthalates came in \
                 with Delegated Directive (EU) 2015/863, so a check built from \
                 the original is four substances short. Not a passport \
                 instrument: the word 'passport' does not occur in the \
                 consolidated text.",
        basis: CitationBasis::Sourced,
    },
    CitedNotModelled {
        celex: "32017L1132",
        reason: "Company law directive. Cited because Implementing Regulation (EU) \
                 2026/2, Annex I note (b), names the European unique identifier \
                 (EUID) 'established by' it, and the unsold-goods schema follows \
                 that wording. What the Directive does, in Art. 16(1), is require \
                 Member States to 'ensure that companies have a European unique \
                 identifier', refer to it by point (8) of the Annex to \
                 Implementing Regulation (EU) 2015/884, and set its minimum \
                 content: elements identifying the Member State of the register, \
                 the domestic register of origin and the company number in it, \
                 and where appropriate features to avoid identification errors. \
                 So the identifier's exact form is not in this Directive.",
        basis: CitationBasis::Sourced,
    },
    CitedNotModelled {
        celex: "32020R0740",
        reason: "Tyre labelling. Cited for Annex I, 'Testing, grading and \
                 measurement of tyre parameters': Parts A and B grade fuel \
                 efficiency (from the rolling resistance coefficient) and wet \
                 grip (from the wet grip index) on A to E scales, Part C sets \
                 the external rolling noise class and measured value, and Parts \
                 D and E add the snow and ice grip pictograms. A labelling \
                 regime, not a passport: the word 'passport' does not occur in \
                 it. It does carry a data duty, discharged through the product \
                 database established under Art. 12 of Regulation (EU) 2017/1369 \
                 rather than through a passport: Art. 5(1) and Annex VII have the \
                 supplier enter the label, its class parameters and the product \
                 information sheet before placing a tyre produced after 1 May \
                 2021 on the market, and \
                 Art. 5(7) keeps a tyre type's data in the compliance part for \
                 five years after its last unit is placed.",
        basis: CitationBasis::Sourced,
    },
    CitedNotModelled {
        celex: "32023R1669",
        reason: "Energy labelling for smartphones and slate tablets, applying \
                 from 20 June 2025 (Art. 8). The sibling of Regulation (EU) \
                 2023/1670: the instrument modelled for that act lists this one \
                 in its legal basis but is keyed on 2023/1670's CELEX number, so \
                 this act needs its own entry here. It sets label content and a \
                 product-database duty, not passport content: Art. 3(1) has the \
                 supplier supply a printed label, enter the product information \
                 sheet parameters (Annex V) and the technical documentation \
                 (Annex VI) in the product database, and the word 'passport' \
                 does not occur in it. Two of its parts matter beyond \
                 labelling, because the electronics schema's repairability \
                 inputs are the operands of the index it defines: Annex II \
                 point C grades the repairability class from the repairability \
                 index R (Table 4), and Annex IV point 5 gives the method for R.",
        basis: CitationBasis::Sourced,
    },
];
