//! Discarded unsold consumer products — ESPR Art. 24 disclosure, in the format
//! of Commission Implementing Regulation (EU) 2026/2 Annex I.
//!
//! # What this can and cannot determine
//!
//! **No passport obligation exists here.** ESPR Arts. 24–25 bind an economic
//! operator over a financial year and require no digital product passport at
//! all. So this never emits a *passport* compliance verdict; it checks that a
//! disclosure is internally consistent with the format the act prescribes, which
//! is a different and much narrower claim.
//!
//! The checks are the ones a single document can answer:
//!
//! - every line's waste-treatment split totals 100% — Annex I note (i) provides
//!   `unknown` for the share that could not be established, so nothing is left
//!   over;
//! - point (h) of Del. Reg. (EU) 2026/296 Art. 2 — offered for donation and not
//!   accepted — is not claimed for a CN category that also claims one of points
//!   (a) to (g), because (h) applies "only where none of" those does;
//! - both narrative rows are present.
//!
//! What it deliberately does **not** check is whether the reason claimed is
//! *true*. Art. 3 of the same act makes that a documentary question — five years
//! of per-derogation evidence, produced to a competent authority within 30 days
//! — and no amount of reading the disclosure answers it.

use dpp_plugin_sdk::traits::{PluginComplianceStatus, PluginResult};
use dpp_plugin_sdk::validate::str_of;
use serde_json::{Value, json};

/// Sum of one line's five treatment shares, widened so a malformed record
/// cannot wrap into a plausible number.
fn treatment_total(line: &Value) -> u32 {
    ["preparingForReusePct", "recyclingPct", "otherRecoveryPct", "disposalPct", "unknownPct"]
        .iter()
        .map(|k| {
            line.get("treatment")
                .and_then(|t| t.get(*k))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32
        })
        .sum()
}

/// The Art. 2 point letter a reason maps to, for the point (h) subordination
/// check. `None` for a reason this build does not know.
fn reason_point(reason: &str) -> Option<char> {
    Some(match reason {
        "dangerousProduct" => 'a',
        "nonCompliantWithLaw" => 'b',
        "intellectualPropertyInfringement" => 'c',
        "licensedPeriodExpired" => 'd',
        "markingsCannotBeRemoved" => 'e',
        "damagedOrContaminated" => 'f',
        "defectiveBeyondRepair" => 'g',
        "offeredForDonationNotAccepted" => 'h',
        "donatedButNoRecipientFound" => 'i',
        "reusedButNoRecipientFound" => 'j',
        _ => return None,
    })
}

pub fn calculate(input: &Value) -> PluginResult {
    let lines = input.get("lines").and_then(Value::as_array);

    let Some(lines) = lines else {
        return PluginResult::new(PluginComplianceStatus::NonCompliant).with_extra(json!({
            "regulationArticle": "ESPR Article 24; Impl. Reg. (EU) 2026/2 Annex I",
            "detail": "disclosure carries no lines",
        }));
    };
    if lines.is_empty() {
        return PluginResult::new(PluginComplianceStatus::NonCompliant).with_extra(json!({
            "regulationArticle": "ESPR Article 24; Impl. Reg. (EU) 2026/2 Annex I",
            "detail": "disclosure carries no lines",
        }));
    }

    let mut problems: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let total = treatment_total(line);
        if total != 100 {
            problems.push(format!(
                "line {i}: waste treatment shares total {total}%, not 100%"
            ));
        }
    }

    // Point (h) is the one derogation defined by the absence of the others, so
    // it has to be checked across the lines of a category, never on one line.
    for (i, line) in lines.iter().enumerate() {
        let reason = str_of(line, "reason").unwrap_or("");
        if reason_point(reason) != Some('h') {
            continue;
        }
        let category = line
            .get("cnCategories")
            .and_then(Value::as_array)
            .and_then(|c| c.first())
            .and_then(Value::as_str)
            .unwrap_or("");
        let clashes = lines.iter().any(|other| {
            let other_category = other
                .get("cnCategories")
                .and_then(Value::as_array)
                .and_then(|c| c.first())
                .and_then(Value::as_str)
                .unwrap_or("");
            other_category == category
                && matches!(
                    reason_point(str_of(other, "reason").unwrap_or("")),
                    Some('a'..='g')
                )
        });
        if clashes {
            problems.push(format!(
                "line {i}: category '{category}' claims Art. 2 point (h) alongside a point (a)-(g) \
                 reason; (h) applies only where none of (a) to (g) does"
            ));
        }
    }

    for (field, note) in [("measuresTaken", 'i'), ("measuresPlanned", 'j')] {
        if str_of(input, field).unwrap_or("").trim().is_empty() {
            problems.push(format!("{field} is required by Annex I note ({note})"));
        }
    }

    let status = if problems.is_empty() {
        PluginComplianceStatus::Compliant
    } else {
        PluginComplianceStatus::NonCompliant
    };

    PluginResult::new(status).with_extra(json!({
        "regulationArticle": "ESPR Article 24; Impl. Reg. (EU) 2026/2 Annex I; Del. Reg. (EU) 2026/296 Art. 2",
        "lineCount": lines.len(),
        "detail": if problems.is_empty() {
            "disclosure is internally consistent with the Annex I format".to_owned()
        } else {
            problems.join("; ")
        },
        "passportObligation": "none — ESPR Arts. 24-25 impose no digital product passport",
    }))
}
