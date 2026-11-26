//! The battery submodels.

use super::super::*;
use super::common::*;
use dpp_domain::Audience;
use dpp_domain::{ProductGroup, ProductGroupData};

#[test]
fn build_aas_with_battery_product_group_data_adds_sixth_submodel() {
    let mut passport = minimal_passport(ProductGroup::Battery);
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(
        battery_data_with_due_diligence(),
    )));

    let (shell, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
    assert_eq!(
        submodels.len(),
        6,
        "battery product_group data should add a 6th submodel"
    );
    assert_eq!(shell.submodels.len(), 6);

    let battery_sub = submodels
        .iter()
        .find(|s| s.id_short == "BatteryTechnicalData");
    assert!(
        battery_sub.is_some(),
        "BatteryTechnicalData submodel missing"
    );

    let battery_sub = battery_sub.unwrap();
    let has_chemistry = battery_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Property(p) => p.id_short == "batteryChemistry",
        _ => false,
    });
    assert!(has_chemistry, "batteryChemistry property missing");

    let has_co2e = battery_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Property(p) => p.id_short == "co2ePerUnitKg",
        _ => false,
    });
    assert!(has_co2e, "co2ePerUnitKg property missing");

    // `dueDiligenceUrl` **must** reach a public projection: Annex XIII point
    // 1(d), "information on responsible sourcing as indicated in the report on
    // battery due diligence policy referred to in Article 52(3)", sits in the
    // publicly accessible tier.
    //
    // This assertion has now been written both ways round, which is the reason
    // it carries its citation. It first asserted presence, was flipped to
    // absence when the field was classified `restricted`, and is flipped back
    // here because that classification did not match the annex. Withholding it
    // is not the safe direction — it makes the public passport omit content the
    // regulation requires to be in it.
    let has_due_diligence_ref = battery_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::ReferenceElement(r) => r.id_short == "dueDiligenceUrl",
        _ => false,
    });
    assert!(
        has_due_diligence_ref,
        "Annex XIII point 1(d) is public: dueDiligenceUrl must appear in a public projection"
    );
}

/// The same field is present for a caller entitled to it — proof the masking is
/// per-audience rather than a blanket strip of everything non-public.
#[test]
fn restricted_audience_receives_the_restricted_battery_field() {
    let mut passport = minimal_passport(ProductGroup::Battery);
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(
        battery_data_with_due_diligence(),
    )));

    let (_, submodels) = build_aas_from_passport(
        &passport,
        identity_of(&passport),
        Audience::LegitimateInterest,
    )
    .expect("buildable");

    let battery_sub = submodels
        .iter()
        .find(|s| s.id_short == "BatteryTechnicalData")
        .expect("battery submodel present");

    assert!(
        battery_sub.submodel_elements.iter().any(|e| match e {
            AasSubmodelElement::ReferenceElement(r) => r.id_short == "dueDiligenceUrl",
            _ => false,
        }),
        "a legitimate-interest caller must still receive dueDiligenceUrl"
    );
}
