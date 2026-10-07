//! Who sees a statement about personal data: exactly the audiences that see the
//! field it is about, and never the public.

use crate::access::{ProductGroupAccessPolicy, filter_by_audience, redact_passport};
use crate::disclosure::Audience;
use crate::passport::Passport;
use crate::personal_data::{HeldOutside, LawfulBasis, PersonalDataRecordId, PersonalDataStatement};
use crate::product_group::{ProductGroup, ProductGroupData};

fn with_statement(product_group: ProductGroup, version: &str, field: &str) -> Passport {
    let data = match product_group {
        ProductGroup::Battery => {
            ProductGroupData::Battery(Box::new(crate::test_support::sample_battery_data()))
        }
        _ => ProductGroupData::Textile(Box::new(crate::test_support::sample_textile_data())),
    };
    let mut passport = Passport {
        product_group,
        product_group_data: Some(data),
        schema_version: version.into(),
        ..crate::test_support::sample_passport()
    };
    passport.personal_data.insert(
        field.into(),
        PersonalDataStatement::HeldOutside(HeldOutside::new(
            LawfulBasis::Consent,
            PersonalDataRecordId::new("rec-1"),
        )),
    );
    passport
}

fn sees_statement(passport: &Passport, audience: Audience) -> bool {
    redact_passport(passport, audience)
        .0
        .get("personalData")
        .is_some()
}

/// Battery accidents are Annex XIII point 4 data: a legitimate interest sees
/// the statement about them, and an authority does not, as it does not see the
/// field.
#[test]
fn a_statement_about_individual_data_goes_only_to_a_legitimate_interest() {
    let passport = with_statement(
        ProductGroup::Battery,
        "2.8.0",
        "usageHistory.negativeEvents",
    );
    assert!(!sees_statement(&passport, Audience::Public));
    assert!(sees_statement(&passport, Audience::LegitimateInterest));
    assert!(!sees_statement(&passport, Audience::Authority));
}

/// The textile repair log is public, and its statement still is not: whether
/// personal data about one item exists is not the public's to learn.
#[test]
fn a_statement_about_a_public_field_is_still_withheld_from_the_public() {
    let passport = with_statement(ProductGroup::Textile, "1.4.0", "repairHistoryUrl");
    assert!(!sees_statement(&passport, Audience::Public));
    assert!(sees_statement(&passport, Audience::LegitimateInterest));
    assert!(sees_statement(&passport, Audience::Authority));
}

/// The statement served is the one stored, under its field's path.
#[test]
fn a_visible_statement_is_served_whole() {
    let passport = with_statement(
        ProductGroup::Battery,
        "2.8.0",
        "usageHistory.negativeEvents",
    );
    let view = redact_passport(&passport, Audience::LegitimateInterest).0;
    assert_eq!(
        view["personalData"],
        serde_json::json!({
            "usageHistory.negativeEvents": {
                "held": "outside", "lawfulBasis": "consent", "record": "rec-1"
            }
        })
    );
}

/// With no policy for the passport's version nothing can say which statements
/// a field's audience covers, so none is served — the same fail-closed answer
/// the product-group data gets.
#[test]
fn an_unknown_schema_version_serves_no_statement() {
    let passport = with_statement(
        ProductGroup::Battery,
        "9.9.9",
        "usageHistory.negativeEvents",
    );
    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        assert!(!sees_statement(&passport, audience), "{audience:?}");
    }
}

/// A consumer driving the raw filter instead of `redact_passport` gets the
/// envelope class, which has to fail safe on its own.
#[test]
fn the_raw_filter_keeps_statements_off_the_public_and_authority_views() {
    let passport = with_statement(
        ProductGroup::Battery,
        "2.8.0",
        "usageHistory.negativeEvents",
    );
    let doc = serde_json::to_value(&passport).unwrap();
    let policy = ProductGroupAccessPolicy::passport_default();
    for audience in [Audience::Public, Audience::Authority] {
        let view = filter_by_audience(&doc, &policy, audience).filtered_data;
        assert!(view.get("personalData").is_none(), "{audience:?}");
    }
}
