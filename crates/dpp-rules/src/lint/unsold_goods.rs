//! Unsold-goods plausibility lints (EU ESPR Article 25 destruction ban
//! reports) — consistency checks the schema does not itself require.

use alloc::{format, vec::Vec};

use super::{LintFinding, LintSeverity};

/// Borrowing view over the unsold-goods report fields these lints inspect.
#[derive(Debug, Clone, Copy)]
pub struct UnsoldGoodsLintInput<'a> {
    pub reporting_period: &'a str,
    pub volume_kg: f64,
    /// Serde code, e.g. `"donation"`, `"exempt_destruction"`.
    pub destination: &'a str,
    pub operator_name: Option<&'a str>,
    pub destruction_justification: Option<&'a str>,
    /// Current UTC year — this crate has no clock, so the caller supplies it.
    pub as_of_year: u32,
    /// Current UTC month (1–12).
    pub as_of_month: u32,
}

struct ParsedPeriod {
    year: u32,
    /// `None` for a bare year; otherwise the last month the period covers
    /// (quarter end for `YYYY-QN`, the month itself for `YYYY-MM`).
    end_month: Option<u32>,
}

fn parse_reporting_period(s: &str) -> Option<ParsedPeriod> {
    let s = s.trim();
    let is_digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());

    if s.len() == 4 && is_digits(s) {
        return Some(ParsedPeriod {
            year: s.parse().ok()?,
            end_month: None,
        });
    }
    if let Some((y, q)) = s.split_once("-Q").or_else(|| s.split_once("-q")) {
        if y.len() == 4 && is_digits(y) && q.len() == 1 && is_digits(q) {
            let quarter: u32 = q.parse().ok()?;
            if (1..=4).contains(&quarter) {
                return Some(ParsedPeriod {
                    year: y.parse().ok()?,
                    end_month: Some(quarter * 3),
                });
            }
        }
        return None;
    }
    if let Some((y, m)) = s.split_once('-') {
        if y.len() == 4 && is_digits(y) && m.len() == 2 && is_digits(m) {
            let month: u32 = m.parse().ok()?;
            if (1..=12).contains(&month) {
                return Some(ParsedPeriod {
                    year: y.parse().ok()?,
                    end_month: Some(month),
                });
            }
        }
        return None;
    }
    None
}

/// Format plausibility: `reportingPeriod` is free text in the schema, but a
/// value that matches none of the conventional forms (`YYYY`, `YYYY-QN`,
/// `YYYY-MM`) used elsewhere in this report is likely a typo.
#[must_use]
pub fn reporting_period_format_implausible(
    input: &UnsoldGoodsLintInput<'_>,
) -> Option<LintFinding> {
    if parse_reporting_period(input.reporting_period).is_some() {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.reporting_period_format_implausible",
        field: "reportingPeriod",
        severity: LintSeverity::Notice,
        message: format!(
            "reportingPeriod '{}' does not match a recognised YYYY, YYYY-QN, or YYYY-MM format — intended?",
            input.reporting_period
        ),
    })
}

/// Cross-field ordering: a disposal cannot be reported for a period that
/// hasn't happened yet. Only fires when the period parses (see
/// [`reporting_period_format_implausible`] for the unparsable case).
#[must_use]
pub fn reporting_period_in_future(input: &UnsoldGoodsLintInput<'_>) -> Option<LintFinding> {
    let period = parse_reporting_period(input.reporting_period)?;
    let is_future = match period.end_month {
        Some(month) => {
            period.year > input.as_of_year
                || (period.year == input.as_of_year && month > input.as_of_month)
        }
        None => period.year > input.as_of_year,
    };
    if !is_future {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.reporting_period_in_future",
        field: "reportingPeriod",
        severity: LintSeverity::Warning,
        message: format!(
            "reportingPeriod '{}' is in the future — intended?",
            input.reporting_period
        ),
    })
}

const VOLUME_KG_IMPLAUSIBLE_THRESHOLD: f64 = 1_000_000.0;

/// Range plausibility: 1,000 tonnes of unsold goods in a single report is far
/// beyond a typical reporting-period volume — worth a second look as a
/// possible unit slip (e.g. grams entered as kilograms).
#[must_use]
pub fn volume_kg_implausibly_large(input: &UnsoldGoodsLintInput<'_>) -> Option<LintFinding> {
    if !input.volume_kg.is_finite() || input.volume_kg <= VOLUME_KG_IMPLAUSIBLE_THRESHOLD {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.volume_kg_implausibly_large",
        field: "volumeKg",
        severity: LintSeverity::Notice,
        message: format!(
            "volumeKg ({}) exceeds 1,000,000 kg for a single report — intended, or a unit slip \
             (e.g. grams entered as kilograms)?",
            input.volume_kg
        ),
    })
}

const THIRD_PARTY_DESTINATIONS: &[&str] = &[
    "donation",
    "recycling",
    "supplier_return",
    "exempt_destruction",
];

/// Claim-without-evidence check: the schema's own field description calls
/// `operatorName` "required for audit trail" for third-party destinations
/// (everything except a same-operator repurposing), yet the field itself is
/// optional.
#[must_use]
pub fn operator_name_missing_for_third_party_destination(
    input: &UnsoldGoodsLintInput<'_>,
) -> Option<LintFinding> {
    let is_third_party = THIRD_PARTY_DESTINATIONS
        .iter()
        .any(|d| input.destination.eq_ignore_ascii_case(d));
    if !is_third_party || input.operator_name.is_some() {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.operator_name_missing_for_third_party_destination",
        field: "operatorName",
        severity: LintSeverity::Notice,
        message: format!(
            "destination '{}' involves a third party but operatorName is absent — intended?",
            input.destination
        ),
    })
}

/// Structural plausibility: `destructionJustification` only has defined
/// meaning when `destination` is `exempt_destruction` (the schema's own
/// conditional-required rule). A populated value alongside any other
/// destination is a stray field, not a schema violation.
#[must_use]
pub fn destruction_justification_without_exempt_destination(
    input: &UnsoldGoodsLintInput<'_>,
) -> Option<LintFinding> {
    if input.destruction_justification.is_none()
        || input.destination.eq_ignore_ascii_case("exempt_destruction")
    {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.destruction_justification_without_exempt_destination",
        field: "destructionJustification",
        severity: LintSeverity::Notice,
        message: format!(
            "destructionJustification is populated but destination is '{}', not exempt_destruction — intended?",
            input.destination
        ),
    })
}

/// Run every unsold-goods plausibility lint and collect the findings.
#[must_use]
pub fn lint_unsold_goods(input: &UnsoldGoodsLintInput<'_>) -> Vec<LintFinding> {
    let mut out = Vec::new();
    out.extend(reporting_period_format_implausible(input));
    out.extend(reporting_period_in_future(input));
    out.extend(volume_kg_implausibly_large(input));
    out.extend(operator_name_missing_for_third_party_destination(input));
    out.extend(destruction_justification_without_exempt_destination(input));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> UnsoldGoodsLintInput<'static> {
        UnsoldGoodsLintInput {
            reporting_period: "2026-Q2",
            volume_kg: 500.0,
            destination: "donation",
            operator_name: Some("Caritas Skopje"),
            destruction_justification: None,
            as_of_year: 2026,
            as_of_month: 7,
        }
    }

    // ── parse_reporting_period (via the two lints that use it) ─────────────

    #[test]
    fn recognised_formats_all_parse() {
        for s in ["2026", "2026-Q2", "2026-07"] {
            let mut input = base_input();
            input.reporting_period = s;
            assert!(reporting_period_format_implausible(&input).is_none(), "{s}");
        }
    }

    #[test]
    fn garbage_format_triggers() {
        let mut input = base_input();
        input.reporting_period = "asdf";
        let finding = reporting_period_format_implausible(&input).unwrap();
        assert_eq!(
            finding.code,
            "unsold_goods.reporting_period_format_implausible"
        );
    }

    #[test]
    fn out_of_range_quarter_triggers_format_lint() {
        let mut input = base_input();
        input.reporting_period = "2026-Q9";
        assert!(reporting_period_format_implausible(&input).is_some());
    }

    // ── reporting_period_in_future ──────────────────────────────────────────

    #[test]
    fn past_period_passes() {
        assert!(reporting_period_in_future(&base_input()).is_none());
    }

    #[test]
    fn future_quarter_triggers() {
        let mut input = base_input();
        input.reporting_period = "2027-Q1";
        let finding = reporting_period_in_future(&input).unwrap();
        assert_eq!(finding.code, "unsold_goods.reporting_period_in_future");
    }

    #[test]
    fn same_year_later_month_triggers() {
        let mut input = base_input();
        input.reporting_period = "2026-12";
        input.as_of_month = 7;
        assert!(reporting_period_in_future(&input).is_some());
    }

    #[test]
    fn unparsable_period_does_not_trigger_future_lint() {
        let mut input = base_input();
        input.reporting_period = "asdf";
        assert!(reporting_period_in_future(&input).is_none());
    }

    // ── volume_kg_implausibly_large ─────────────────────────────────────────

    #[test]
    fn ordinary_volume_passes() {
        assert!(volume_kg_implausibly_large(&base_input()).is_none());
    }

    #[test]
    fn huge_volume_triggers() {
        let mut input = base_input();
        input.volume_kg = 5_000_000.0;
        let finding = volume_kg_implausibly_large(&input).unwrap();
        assert_eq!(finding.code, "unsold_goods.volume_kg_implausibly_large");
    }

    // ── operator_name_missing_for_third_party_destination ───────────────────

    #[test]
    fn third_party_with_operator_name_passes() {
        assert!(operator_name_missing_for_third_party_destination(&base_input()).is_none());
    }

    #[test]
    fn third_party_without_operator_name_triggers() {
        let mut input = base_input();
        input.operator_name = None;
        let finding = operator_name_missing_for_third_party_destination(&input).unwrap();
        assert_eq!(
            finding.code,
            "unsold_goods.operator_name_missing_for_third_party_destination"
        );
    }

    #[test]
    fn repurposing_destination_never_triggers() {
        let mut input = base_input();
        input.destination = "repurposing";
        input.operator_name = None;
        assert!(operator_name_missing_for_third_party_destination(&input).is_none());
    }

    // ── destruction_justification_without_exempt_destination ────────────────

    #[test]
    fn no_justification_passes() {
        assert!(destruction_justification_without_exempt_destination(&base_input()).is_none());
    }

    #[test]
    fn justification_on_exempt_destination_passes() {
        let mut input = base_input();
        input.destination = "exempt_destruction";
        input.destruction_justification =
            Some("Contaminated batch, health authority order 2026-119");
        assert!(destruction_justification_without_exempt_destination(&input).is_none());
    }

    #[test]
    fn justification_on_other_destination_triggers() {
        let mut input = base_input();
        input.destruction_justification = Some("stray text");
        let finding = destruction_justification_without_exempt_destination(&input).unwrap();
        assert_eq!(
            finding.code,
            "unsold_goods.destruction_justification_without_exempt_destination"
        );
    }

    // ── lint_unsold_goods aggregator ────────────────────────────────────────

    #[test]
    fn clean_input_produces_no_findings() {
        assert!(lint_unsold_goods(&base_input()).is_empty());
    }
}
