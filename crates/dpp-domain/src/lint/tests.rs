//! What the plausibility lints report, and what they stay silent about.

use super::*;
use crate::product_group::BatteryData;
use crate::product_group::ProductGroupData;
use crate::product_group::data::unsold_goods::{CnCategory, DiscardReason};
use chrono::Utc;

fn battery() -> BatteryData {
    BatteryData {
        nominal_voltage_v: 3.7,
        nominal_capacity_ah: 10.0,
        expected_lifetime_cycles: Some(500),
        co2e_per_unit_kg: 5.0,
        rated_energy_wh: Some(37.0),
        ..crate::test_support::sample_battery_data()
    }
}

#[test]
fn clean_battery_produces_no_findings() {
    let data = ProductGroupData::Battery(Box::new(battery()));
    assert!(lint_product_group_data(&data, Utc::now()).is_empty());
}

#[test]
fn battery_energy_mismatch_surfaces_as_domain_finding() {
    let mut b = battery();
    b.rated_energy_wh = Some(500.0); // 3.7 * 10.0 = 37.0 expected
    let data = ProductGroupData::Battery(Box::new(b));
    let findings = lint_product_group_data(&data, Utc::now());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "battery.energy_capacity_mismatch");
    assert_eq!(findings[0].severity, LintSeverity::Notice);
}

#[test]
fn a_well_formed_disclosure_produces_no_findings() {
    let data = ProductGroupData::UnsoldGoods(crate::test_support::sample_unsold_goods_report());
    assert_eq!(lint_product_group_data(&data, Utc::now()), Vec::new());
}

/// Annex I note (i) provides `unknown` for the share that could not be
/// established, so a split that does not reach 100 has lost weight rather
/// than being unsure about it.
#[test]
fn a_treatment_split_that_misses_100_is_flagged() {
    let mut report = crate::test_support::sample_unsold_goods_report();
    report.lines[0].treatment.disposal_pct = 1;
    let data = ProductGroupData::UnsoldGoods(report);
    let findings = lint_product_group_data(&data, Utc::now());
    assert!(
        findings
            .iter()
            .any(|f| f.code == "unsold_goods.treatment_split_does_not_total_100"),
        "{findings:?}"
    );
}

/// Art. 3 requires four digits for Annex II products, and chapter 85 holds
/// several — so a chapter-level line there hides which heading applied.
#[test]
fn a_chapter_holding_annex_ii_headings_is_flagged() {
    let mut report = crate::test_support::sample_unsold_goods_report();
    report.lines[0].cn_categories = vec![CnCategory::parse("85").expect("valid chapter")];
    let data = ProductGroupData::UnsoldGoods(report);
    let findings = lint_product_group_data(&data, Utc::now());
    assert!(
        findings
            .iter()
            .any(|f| f.code == "unsold_goods.cn_category_needs_four_digits"),
        "{findings:?}"
    );
}

/// Point (h) applies "only where none of the circumstances referred to in
/// points (a) to (g) are applicable", so it cannot sit beside one of them
/// for the same category.
#[test]
fn donation_claimed_beside_a_stronger_reason_is_flagged() {
    let mut report = crate::test_support::sample_unsold_goods_report();
    let mut second = report.lines[0].clone();
    second.reason = DiscardReason::OfferedForDonationNotAccepted;
    report.lines.push(second);
    let data = ProductGroupData::UnsoldGoods(report);
    let findings = lint_product_group_data(&data, Utc::now());
    assert!(
        findings
            .iter()
            .any(|f| f.code == "unsold_goods.donation_reason_alongside_stronger_reason"),
        "{findings:?}"
    );
}

#[test]
fn other_product_group_produces_no_findings() {
    let data = ProductGroupData::other(serde_json::json!({"productGroup": "packaging"}))
        .expect("packaging has no typed variant");
    assert!(lint_product_group_data(&data, Utc::now()).is_empty());
}

#[test]
fn lint_result_compute_stamps_pack_version_and_timestamp() {
    let data = ProductGroupData::Battery(Box::new(battery()));
    let result = LintResult::compute(&data);
    assert_eq!(result.pack_version, dpp_rules::lint::LINT_PACK_VERSION);
    assert!(result.findings.is_empty());
}

#[test]
fn lint_result_serde_round_trip() {
    let data = ProductGroupData::Battery(Box::new(battery()));
    let result = LintResult::compute(&data);
    let json = serde_json::to_value(&result).unwrap();
    let back: LintResult = serde_json::from_value(json).unwrap();
    assert_eq!(back, result);
}

// ── SVHC, the cross-product-group pack ───────────────────────────────────────

fn svhc(cas: &str, name: &str, pct: f64) -> crate::product_group::SvhcSubstance {
    crate::product_group::SvhcSubstance {
        cas_number: cas.to_owned(),
        substance_name: name.to_owned(),
        concentration_pct: pct,
        location_in_product: None,
        scip_notification_id: None,
    }
}

/// The point of the change: the candidate list is consulted at all. Before this,
/// `check_svhc_declarations` had no caller and a declared SVHC above the Art. 33
/// threshold was indistinguishable from any other well-formed entry.
#[test]
fn a_declared_svhc_above_the_threshold_reaches_the_passport_as_a_finding() {
    let mut t = crate::test_support::sample_textile_data();
    t.svhc_substances = Some(vec![svhc("80-05-7", "Bisphenol A", 0.5)]);
    let data = ProductGroupData::Textile(Box::new(t));

    let codes: Vec<String> = lint_product_group_data(&data, Utc::now())
        .into_iter()
        .map(|f| f.code)
        .collect();
    assert!(
        codes
            .iter()
            .any(|c| c == "svhc.article_33_communication_owed"),
        "expected the Art. 33 notice, got {codes:?}"
    );
}

/// The lint runs on product groups that have no pack of their own — which is why
/// it sits outside the product-group match rather than inside five arms.
#[test]
fn the_svhc_pack_reaches_a_product_group_with_no_pack_of_its_own() {
    let toy = crate::product_group::ToyData {
        gtin: crate::Gtin::parse("09506000134352").unwrap(),
        age_group: "3-6".to_owned(),
        primary_material: "plastic".to_owned(),
        ce_marking: true,
        country_of_origin: "DE".to_owned(),
        svhc_substances: Some(vec![svhc("117-81-7", "DEHP", 1.0)]),
        contains_battery: None,
        repairability_info: None,
    };
    let data = ProductGroupData::Toy(toy);

    let codes: Vec<String> = lint_product_group_data(&data, Utc::now())
        .into_iter()
        .map(|f| f.code)
        .collect();
    assert!(
        codes
            .iter()
            .any(|c| c == "svhc.article_33_communication_owed"),
        "toy has no lint pack; the SVHC checks must still run: {codes:?}"
    );
}

/// A group whose schema never asks for the declaration must not be rendered as
/// having answered it. `None` and `Some(&[])` are different claims.
#[test]
fn a_group_that_declares_no_substances_field_produces_no_svhc_findings() {
    let data = ProductGroupData::Battery(Box::new(battery()));
    assert!(
        !lint_product_group_data(&data, Utc::now())
            .iter()
            .any(|f| f.code.starts_with("svhc.")),
        "battery carries no svhcSubstances field, so it was never asked"
    );
}

/// The completeness caveat is the finding that makes list currency visible.
#[test]
fn the_partial_list_caveat_is_attached_when_anything_was_checked() {
    let mut t = crate::test_support::sample_textile_data();
    // Below the threshold, so the only finding left is the caveat itself.
    t.svhc_substances = Some(vec![svhc("80-05-7", "Bisphenol A", 0.01)]);
    let data = ProductGroupData::Textile(Box::new(t));

    let findings = lint_product_group_data(&data, Utc::now());
    let caveat = findings
        .iter()
        .find(|f| f.code == "svhc.candidate_list_partial")
        .expect("a partial list must qualify its own clean result");
    assert!(
        caveat.message.contains("2026-02-04"),
        "the caveat must name the revision it checked against: {}",
        caveat.message
    );
}
