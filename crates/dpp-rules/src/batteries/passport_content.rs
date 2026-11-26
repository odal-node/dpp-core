//! Which battery passport data points a given battery category actually owes.
//!
//! The schema deliberately declares almost every Annex XIII field optional, and
//! that is not laxity: the obligations are **per category**. A field mandatory
//! for an electric-vehicle battery may be *"not to be filled/displayed"* for an
//! LMT one, and requiring it in JSON Schema would make a lawful industrial
//! battery unrepresentable. The constraint belongs here, where it can be
//! expressed as a function of the category.
//!
//! # Source
//!
//! The Commission's *Guidance Document: Digital Batteries Passport — data
//! points by category* (v2.0, 15 August 2026). The table was first read in full
//! against v1.0 (28 July 2026); v2.0 was then compared with v1.0 line by line.
//! Its obligations differ in rows 19 to 23 only, which are taken up below, and
//! the rest of its changes are wording: rows 6 and 25 now give the reason they
//! are not filled, row 35 drops an "only", and a footnote defines "if
//! applicable". Its table has one row per data point and one column per
//! category, which is the shape of the `REQUIREMENTS` table below — so a
//! reviewer can diff the two directly rather than reconstructing the mapping.
//!
//! Version 3.2 (19 October 2026) was compared with this table on 2 November 2026
//! and changes none of its obligations. It splits the C-rate of the cycle-life
//! test, point 1(p), into a mandatory charge C-rate, whose obligations
//! `cycleLifeTestCRate` carries, and a discharge C-rate that is optional for every
//! category and has no field. That renumbers rows between 40 and 50, so the row
//! numbers cited here are 2.0's. It also adds a column for the type of
//! information, which agrees with the battery schema's disclosure classes.
//!
//! v2.0 also states its industrial scope as batteries with a capacity greater
//! than 2 kWh. This module does not see capacity, so it answers for the
//! `industrial` category as a whole.
//!
//! **The guidance covers EV, LMT and industrial batteries only.** It says
//! nothing about portable or SLI batteries, so this module answers
//! [`Requirement::Unknown`] for them rather than guessing. Silence in the source
//! is not permission, and it is not prohibition either.
//!
//! # What this table is not
//!
//! It is **not** a row-for-row copy of the guidance's 71 data points, and the
//! two cannot be diffed on length. This table keys on the wire names of
//! *product group-data* fields, so a guidance row whose content lives on the passport
//! envelope has no row here and is enforced elsewhere:
//!
//! - Data points 3 and 4 (manufacturer name; postal address) are
//!   `manufacturer.name` and `manufacturer.address`, which the domain's own
//!   `Passport::validate` already requires to be non-empty.
//! - Data point 2 (identity of who is registering or is responsible for the
//!   passport) is carried by the responsible-operator and operator-identifier
//!   envelope fields.
//!
//! An omission here is therefore only meaningful for a data point that *does*
//! map to a product group-data field.
//!
//! # Where all 71 rows went
//!
//! Every guidance row has been walked against the model. The difference between
//! 71 and this table's length is accounted for as follows, so a future reader
//! can check the claim rather than take it:
//!
//! - **Envelope, not product group data** — rows 2, 3, 4 (registrant identity;
//!   manufacturer name; postal address), enforced as above. Row 5
//!   (manufacturer web and email) is *"optional, to be filled if such data is
//!   available"*, so it carries no obligation to enforce.
//! - **One block, many rows** — rows 51–60 are the ten members of
//!   `DynamicPerformance`, rows 61–66 the two disjoint `StateOfHealth`
//!   parameter sets, and rows 68–71 the four members of `UsageHistory`. Each
//!   block is one row here because the block is what a passport carries or
//!   omits; the per-member conditions live in the types.
//! - **Deferred by the guidance** — rows 17 to 23 and 44 are *"not to be
//!   filled/displayed as of February 2027"*. The six with a modelled field
//!   (`carbonFootprintClass`, `dueDiligenceUrl`, and the four recycled-content
//!   shares of rows 20 to 23) are listed below as `NotApplicable`; rows 17 and
//!   44 (carbon footprint *declaration*, and instructions for use) have no field
//!   at all, which is the correct state for a data point whose format has not
//!   been specified. `co2ePerUnitKg` is barred beside row 17: it is not the
//!   declaration, but a figure labelled carbon footprint in a passport states one
//!   the guidance says not to display.
//! - **Restated elsewhere in the annex** — row 16 is Annex XIII point 1(c)
//!   material composition, deferred, but its constituents are separately
//!   mandatory as Annex VI Part A points 7, 8 and 10 (rows 12, 13, 15) and are
//!   enforced there.
//!
//! Four omissions were found and closed by this audit's first pass: points 1,
//! 7, 8 and 9 — the unique identifier, model identification, place of
//! manufacture and date of manufacture — all mandatory for every covered
//! category, and none of which any battery schema version had ever declared.
//!
//! # The table is a snapshot; one row moves
//!
//! `REQUIREMENTS` records each obligation as of February 2027, which is what the
//! guidance states, and it stays that way so the two can still be diffed. A row
//! whose answer changes on a date, or turns on facts about the battery, is
//! resolved beside the table rather than in it. There is one: row 19,
//! `dueDiligenceUrl`, which Art. 48(1) makes due from 18 August 2027, subject to
//! Art. 47's two exemptions (see [`super::due_diligence`]). Every function here
//! therefore takes the [`BatteryFacts`] such a row reads, and a row with nothing
//! scheduled ignores them.

use super::battery_facts::BatteryFacts;
use super::due_diligence::{DueDiligenceScope, due_diligence_scope};

/// What a category owes for one data point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// The guidance marks it mandatory. A published passport without it is
    /// missing content the law requires.
    Mandatory,
    /// *"If applicable"*, *"where possible"*, or *"only applicable for some
    /// industrial batteries"* — the duty exists but its trigger is a fact about
    /// the individual battery that no schema can decide.
    Conditional,
    /// *"Not to be filled/displayed"*. Present in a passport of this category,
    /// the value is not merely surplus — the guidance says it does not belong.
    NotApplicable,
    /// The guidance does not cover this category, or does not name this field.
    /// Distinct from [`Self::NotApplicable`]: one is a recorded exclusion, the
    /// other is an absence of evidence.
    Unknown,
    /// The law takes this battery out of the duty, and the field is not barred.
    ///
    /// Distinct from [`Self::NotApplicable`], which says the field does not
    /// belong in a passport of this category at all. Here the duty exists for
    /// the category and this battery is excused from it, so a value supplied
    /// anyway is not wrong.
    Exempt,
    /// The answer turns on a fact about this battery that the caller did not
    /// supply, and the variant names it.
    ///
    /// Distinct from [`Self::Unknown`]: that one says nobody has read the
    /// guidance against this field; this one says the rule is known and the
    /// input it needs is missing. **Not an exemption**, and a caller must not
    /// treat it as one.
    Undetermined(MissingFact),
}

impl Requirement {
    /// Whether a passport of this category may carry the field at all.
    #[must_use]
    pub const fn permits_presence(self) -> bool {
        !matches!(self, Self::NotApplicable)
    }
}

/// A fact a [`Requirement::Undetermined`] answer is waiting on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingFact {
    /// The date the battery was placed on the market or put into service.
    PlacingDate,
    /// The operator's declaration against Art. 47, first paragraph
    /// ([`OperatorScope`](super::due_diligence::OperatorScope)).
    OperatorScope,
}

impl MissingFact {
    /// The wire name of the product group-data field that supplies the fact.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::PlacingDate => "placedOnMarketDate",
            Self::OperatorScope => "dueDiligenceOperatorScope",
        }
    }
}

/// The category a rule is being asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Ev,
    Lmt,
    Industrial,
}

fn category_of(battery_type: &str) -> Option<Category> {
    let t = battery_type.trim();
    let eq = |s: &str| t.eq_ignore_ascii_case(s);
    if eq("ev") {
        Some(Category::Ev)
    } else if eq("lmt") {
        Some(Category::Lmt)
    } else if eq("industrial") {
        Some(Category::Industrial)
    } else {
        // portable, sli / starting-lighting-ignition, unknown — outside the
        // guidance's scope entirely.
        None
    }
}

use Requirement::{Conditional as C, Mandatory as M, NotApplicable as X};

/// One row per data point: the wire field name, then EV, LMT, industrial.
///
/// Ordered to follow the Commission's own numbering, so the two can be read
/// side by side. Fields the guidance marks mandatory for all three are included
/// rather than defaulted, because "we checked and it is uniform" and "we did not
/// check" must not look the same in this table.
const REQUIREMENTS: &[(&str, Requirement, Requirement, Requirement)] = &[
    // ── Art. 77(3) — the identifier the passport is reached by ─────────────
    // Guidance data point 1, "unique identifier", mandatory for all three —
    // and deliberately **not a row**. This table checks product-group data,
    // and the identifier is not there: it is the passport's data carrier
    // identifier, the product identifier together with the carrier serial the
    // operator attributes on the envelope, which is what the QR code links to.
    // A `batteryPassportNumber` field required here would be a second home for
    // one identifier, free to disagree with the one the label actually carries.
    // ── Annex VI Part A, reached by Annex XIII point 1(a) ──────────────────
    ("batteryType", M, M, M),
    // Guidance data point 7 — "model identification and batch or serial
    // number, or product number or another element allowing their
    // identification", the second half of Part A point 2.
    ("batteryModelId", M, M, M),
    // Guidance data point 8 — Part A point 3.
    ("manufacturingPlace", M, M, M),
    // Guidance data point 9 — Part A point 4, "month and year".
    ("manufacturingDate", M, M, M),
    ("batteryWeightKg", M, M, M),
    ("nominalCapacityAh", M, M, M),
    ("batteryChemistry", M, M, M),
    ("hazardousSubstances", M, M, M),
    ("usableExtinguishingAgent", M, M, M),
    ("criticalRawMaterials", M, M, M),
    // ── Annex XIII point 1 ─────────────────────────────────────────────────
    // Guidance rows 17 to 23 are *"not to be filled/displayed as of February
    // 2027"* for all three categories, each with its own reason:
    //
    // - Rows 17 and 18, the carbon footprint: the format is still to be
    //   specified in the upcoming implementing act.
    // - Row 19, the due diligence report: as provided in Article 48(1),
    //   required from August 2027. Art. 48(1), as replaced by Reg. (EU)
    //   2025/1561, applies from 18 August 2027. v1.0 gave the implementing-act
    //   reason here too; v2.0 names the date instead. This table records the
    //   obligation as of February 2027, so the row stays `NotApplicable`, and
    //   `scheduled` answers for it instead: not to be filled before that date,
    //   then owed unless Art. 47 exempts the battery. It is not a plain
    //   `Mandatory` from that date, because Art. 47 takes two cases out of the
    //   chapter that the guidance does not mention.
    // - Rows 20 to 23, the shares of cobalt, lithium, nickel and lead recovered
    //   from waste: to be applied in line with Article 8 and the relevant
    //   delegated act. v1.0 had them `Mandatory` for all three.
    //
    // `NotApplicable` is the honest class for each, and it is the same one
    // `ratedCapacityAh` carries below. It is not a statement that the field is
    // wrong; it is the guidance saying a value filed today cannot be the one
    // that will be asked for.
    //
    // `co2ePerUnitKg` is not a guidance row. It is barred beside row 17
    // because a per-unit figure labelled carbon footprint states a footprint
    // the guidance says not to display, in a unit that is not the one
    // Article 7(1)(d) sets for the declaration (per kWh of total energy over the
    // expected service life).
    //
    // Distinct from disclosure: `dueDiligenceUrl` is Annex XIII point 1(d) and
    // therefore *public* when it is eventually filled. What may be seen and
    // whether it may be filled are different axes, and this table only answers
    // the second.
    ("co2ePerUnitKg", X, X, X),
    ("carbonFootprintClass", X, X, X),
    ("dueDiligenceUrl", X, X, X),
    ("recycledContentCobaltPct", X, X, X),
    ("recycledContentLithiumPct", X, X, X),
    ("recycledContentNickelPct", X, X, X),
    ("recycledContentLeadPct", X, X, X),
    // Row 24, point 1(f), is mandatory in both versions.
    ("renewableContentPct", M, M, M),
    // Point 1(g) is "not to be filled/displayed" for every category. The Ah
    // figure a passport does carry is Annex VI Part A point 6, above.
    ("ratedCapacityAh", X, X, X),
    ("minimalVoltageV", M, M, M),
    ("nominalVoltageV", M, M, M),
    ("maximumVoltageV", M, M, M),
    ("originalPowerCapabilityW", M, M, M),
    ("powerLimitMinW", M, M, M),
    ("powerLimitMaxW", M, M, M),
    // 1(j): "only applicable for some industrial batteries where lifetime can
    // be expressed in cycles".
    ("expectedLifetimeCycles", M, M, C),
    ("expectedLifetimeReferenceTest", M, M, C),
    // 1(k): mandatory for EV only, and explicitly not for the other two.
    ("capacityThresholdForExhaustionPct", M, X, X),
    ("notInUseTemperatureRange", M, M, M),
    ("notInUseTemperatureReferenceTest", M, M, M),
    // 1(m): "only if applicable (if commercial warranty envisaged)".
    ("commercialWarrantyPeriodMonths", C, C, C),
    // 1(n): "only applicable for some industrial batteries".
    ("initialRoundTripEfficiencyPct", M, M, C),
    ("roundTripEfficiencyAtHalfCycleLifePct", M, M, C),
    ("internalCellResistanceMohm", M, M, M),
    ("internalPackResistanceMohm", M, M, M),
    ("cycleLifeTestCRate", M, M, C),
    // 1(q), guidance row 40: "the marking requirements laid down in Article
    // 13(4)" — the separate-collection symbol.
    //
    // **Do not "correct" these two against the OJ text of 1(q).** It reads
    // "the marking requirements laid down in Article 13(3) and (4)", while the
    // guidance splits 1(q) into rows 40 and 41 citing 13(4) and 13(5). The two
    // instruments disagree, and this table follows the guidance because that is
    // the source it mirrors and the one that assigns per-category obligations.
    // Art. 13(3) is the non-rechargeable-portable duration label, which is
    // outside the three categories the guidance covers at all.
    ("markingInformation", M, M, M),
    // 1(q), guidance row 41, Art. 13(5): "cadmium or lead symbol if
    // applicable" — the same words for all three categories.
    ("hazardSymbol", C, C, C),
    ("euDeclarationOfConformity", M, M, M),
    ("wasteBatteryInformation", M, M, M),
    // ── Annex XIII points 2 and 3 ──────────────────────────────────────────
    ("cathodeMaterial", M, M, M),
    ("anodeMaterial", M, M, M),
    ("electrolyteMaterial", M, M, M),
    ("componentPartNumbers", M, M, M),
    ("sparePartsContacts", M, M, M),
    ("disassemblyInstructionsUrl", M, M, M),
    ("safetyMeasures", M, M, M),
    ("testReportResults", M, M, M),
    // ── Annex XIII point 4 — the individual-battery tier ───────────────────
    // 4(a): mandatory for EV and LMT, "if applicable" for industrial.
    ("dynamicPerformance", M, M, C),
    // 4(b): the two disjoint state-of-health lists. Which parameter set applies
    // is `degradation::annex_vii_parameter_set_for`; that the block is owed at
    // all is here.
    ("stateOfHealth", M, M, C),
    ("batteryStatus", M, M, M),
    // 4(d): "if applicable" for every category.
    ("usageHistory", C, C, C),
];

/// The answer for a row the table cannot give alone, or `None` for a row it can.
///
/// One row today: guidance row 19, `dueDiligenceUrl`, for all three categories.
/// The answer is the same for each because Art. 48(1) names "batteries" without
/// distinguishing them.
fn scheduled(field: &str, facts: BatteryFacts) -> Option<Requirement> {
    if field != "dueDiligenceUrl" {
        return None;
    }
    let scope = due_diligence_scope(
        facts.placed_on_market,
        facts.second_life,
        facts.operator_scope,
    );
    Some(match scope {
        DueDiligenceScope::Required => Requirement::Mandatory,
        DueDiligenceScope::NotYetBinding => Requirement::NotApplicable,
        DueDiligenceScope::ExemptSecondLife | DueDiligenceScope::ExemptBelowTurnoverThreshold => {
            Requirement::Exempt
        }
        DueDiligenceScope::PlacingDateUnknown => {
            Requirement::Undetermined(MissingFact::PlacingDate)
        }
        DueDiligenceScope::OperatorScopeUndeclared => {
            Requirement::Undetermined(MissingFact::OperatorScope)
        }
    })
}

/// What a battery of `battery_type` owes for the passport field `field`.
///
/// `field` is the **wire** name — `expectedLifetimeCycles`, not
/// `expected_lifetime_cycles` — because this crate is `no_std` and zero-dep and
/// is consumed both by the domain types and by the Wasm product group plugins, which
/// see JSON and never the Rust struct.
///
/// `facts` is read only by a row whose answer is scheduled (see the module
/// documentation); every other row ignores it, so a caller with no facts to give
/// passes [`BatteryFacts::default()`] and loses nothing but those rows' answers.
///
/// Returns [`Requirement::Unknown`] for a category the guidance does not cover
/// (portable, SLI) and for a field it does not name. A caller must not read
/// that as permission or as prohibition — it means nobody has checked.
#[must_use]
pub fn annex_xiii_requirement(field: &str, battery_type: &str, facts: BatteryFacts) -> Requirement {
    let Some(category) = category_of(battery_type) else {
        return Requirement::Unknown;
    };
    let mut i = 0;
    while i < REQUIREMENTS.len() {
        let (name, ev, lmt, ind) = REQUIREMENTS[i];
        if name.as_bytes() == field.as_bytes() {
            if let Some(r) = scheduled(name, facts) {
                return r;
            }
            return match category {
                Category::Ev => ev,
                Category::Lmt => lmt,
                Category::Industrial => ind,
            };
        }
        i += 1;
    }
    Requirement::Unknown
}

/// Every row's answer for this category, in table order. Empty for a category
/// the guidance does not cover.
fn requirements(
    battery_type: &str,
    facts: BatteryFacts,
) -> impl Iterator<Item = (&'static str, Requirement)> {
    let category = category_of(battery_type);
    REQUIREMENTS.iter().filter_map(move |(name, ev, lmt, ind)| {
        let table = match category? {
            Category::Ev => *ev,
            Category::Lmt => *lmt,
            Category::Industrial => *ind,
        };
        Some((*name, scheduled(name, facts).unwrap_or(table)))
    })
}

/// Every field this category must carry, in table order.
///
/// The publish gate's input. Iterating the table rather than exposing it keeps
/// the rows private, so a caller cannot come to depend on their order or arity.
pub fn mandatory_fields(
    battery_type: &str,
    facts: BatteryFacts,
) -> impl Iterator<Item = &'static str> {
    requirements(battery_type, facts)
        .filter_map(|(name, r)| (r == Requirement::Mandatory).then_some(name))
}

/// Every field whose answer is [`Requirement::Undetermined`], with the fact it
/// is waiting on, in table order.
///
/// The publish gate refuses on these as well as on missing content: a field
/// whose obligation cannot be decided has not been shown to be optional.
pub fn undetermined_fields(
    battery_type: &str,
    facts: BatteryFacts,
) -> impl Iterator<Item = (&'static str, MissingFact)> {
    requirements(battery_type, facts).filter_map(|(name, r)| match r {
        Requirement::Undetermined(fact) => Some((name, fact)),
        _ => None,
    })
}

/// Every field of `present` that this category must not carry.
///
/// The complement of the usual question. A passport asserting a capacity
/// threshold for exhaustion on an LMT battery is not missing anything — it is
/// carrying content the guidance says does not belong, which a
/// mandatory-fields check would never notice.
pub fn fields_not_applicable<'a>(
    present: &'a [&'a str],
    battery_type: &str,
    facts: BatteryFacts,
) -> impl Iterator<Item = &'a str> {
    present.iter().copied().filter(move |f| {
        annex_xiii_requirement(f, battery_type, facts) == Requirement::NotApplicable
    })
}

#[cfg(test)]
mod tests;
