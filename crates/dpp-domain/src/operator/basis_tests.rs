//! What `ResponsibilityBasis` can say, and that `all()` stays honest.

use super::ResponsibilityBasis;

/// `all()` must list every variant, the same two-stage contract
/// `OperatorRole::ALL` carries: the match is exhaustive with no catch-all, so a
/// new variant stops this compiling, and the length assertion then fails until
/// `all()` is updated.
#[test]
fn all_lists_every_variant() {
    for basis in ResponsibilityBasis::all() {
        match basis {
            ResponsibilityBasis::MarketSurveillanceArt4
            | ResponsibilityBasis::GeneralProductSafety
            | ResponsibilityBasis::OtherUnionLaw { .. } => {}
        }
    }
    assert_eq!(ResponsibilityBasis::all().len(), 3);
}

/// The catch-all limb of Annex III(k) is only useful if it says which law it
/// means. A basis that cannot name its instrument is not a basis.
#[test]
fn the_catch_all_carries_its_citation() {
    let basis = ResponsibilityBasis::OtherUnionLaw {
        citation: "Article 7 of Regulation (EU) 2017/745".into(),
    };
    let json = serde_json::to_value(&basis).unwrap();
    assert_eq!(
        json["otherUnionLaw"]["citation"],
        "Article 7 of Regulation (EU) 2017/745"
    );
    let back: ResponsibilityBasis = serde_json::from_value(json).unwrap();
    assert_eq!(back, basis);
}

#[test]
fn the_two_named_bases_round_trip() {
    for basis in [
        ResponsibilityBasis::MarketSurveillanceArt4,
        ResponsibilityBasis::GeneralProductSafety,
    ] {
        let json = serde_json::to_value(&basis).unwrap();
        let back: ResponsibilityBasis = serde_json::from_value(json).unwrap();
        assert_eq!(back, basis);
    }
}
