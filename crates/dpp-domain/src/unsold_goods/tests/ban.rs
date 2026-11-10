//! When ESPR Art. 25(1) bars a destruction.

use chrono::NaiveDate;

use crate::identifier::CommodityCode;
use crate::unsold_goods::{DESTRUCTION_BAN_FROM, EnterpriseSize, destruction_ban_applies};

/// A calendar date, for readability.
fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

/// Unsold apparel destroyed by a large enterprise after the ban began: the case the
/// ban exists for, and the one every test below changes one thing of.
fn bars(cn_codes: &[&str], size: EnterpriseSize, on: NaiveDate) -> bool {
    destruction_ban_applies(true, &codes(cn_codes), size, on)
}

/// Parsed codes, as the rule takes them.
fn codes(cn_codes: &[&str]) -> Vec<CommodityCode> {
    cn_codes
        .iter()
        .map(|code| CommodityCode::parse(code).expect("a readable code"))
        .collect()
}

/// The ban is for the destruction of unsold products, and a passport does not say
/// whether the product was ever sold.
#[test]
fn a_product_that_was_not_unsold_is_outside_the_ban() {
    let on = date(2027, 3, 14);
    assert!(bars(&["62034231"], EnterpriseSize::Large, on));
    assert!(!destruction_ban_applies(
        false,
        &codes(&["62034231"]),
        EnterpriseSize::Large,
        on
    ));
}

/// Annex VII lists apparel, clothing accessories and footwear, by prefix of a
/// product's own 6, 8 or 10 digit code.
#[test]
fn only_a_product_in_annex_vii_is_barred() {
    let on = date(2027, 3, 14);
    let large = EnterpriseSize::Large;

    assert!(bars(&["610910"], large, on), "chapter 61, t-shirts");
    assert!(bars(&["6403990000"], large, on), "footwear");
    assert!(!bars(&["85076000"], large, on), "a battery is not listed");
    assert!(
        !bars(&["640610"], large, on),
        "parts of footwear are not listed"
    );
    assert!(
        !bars(&[], large, on),
        "no code, nothing to place in Annex VII"
    );
}

/// A padded code is the same code to `CommodityCode::parse`, so it is barred like
/// the trimmed one; a raw prefix match would have let it escape.
#[test]
fn a_padded_code_is_barred_like_the_trimmed_one() {
    assert!(bars(
        &[" 62034231 "],
        EnterpriseSize::Large,
        date(2027, 3, 14)
    ));
}

/// Items sold together can carry several codes (Annex I note (f)); one in Annex VII
/// is enough.
#[test]
fn one_listed_code_among_several_is_enough() {
    assert!(bars(
        &["85076000", "62034231"],
        EnterpriseSize::Large,
        date(2027, 3, 14)
    ));
}

/// Art. 25(1): "From 19 July 2026". The day before is lawful, the day itself is not.
#[test]
fn the_ban_starts_on_19_july_2026() {
    let large = EnterpriseSize::Large;
    let day_before = DESTRUCTION_BAN_FROM.pred_opt().expect("a day before");

    assert!(!bars(&["62034231"], large, day_before));
    assert!(bars(&["62034231"], large, DESTRUCTION_BAN_FROM));
    assert_eq!(DESTRUCTION_BAN_FROM, date(2026, 7, 19));
}

/// The ban does not apply to micro and small enterprises, and applies to
/// medium-sized ones from 19 July 2030.
#[test]
fn the_ban_does_not_reach_an_exempt_operator() {
    let apparel = ["62034231"];
    let on = date(2028, 1, 1);

    assert!(!bars(&apparel, EnterpriseSize::Micro, on));
    assert!(!bars(&apparel, EnterpriseSize::Small, on));
    assert!(!bars(&apparel, EnterpriseSize::Medium, on));
    assert!(!bars(&apparel, EnterpriseSize::Medium, date(2030, 7, 18)));
    assert!(bars(&apparel, EnterpriseSize::Medium, date(2030, 7, 19)));
    assert!(!bars(&apparel, EnterpriseSize::Micro, date(2031, 1, 1)));
}
