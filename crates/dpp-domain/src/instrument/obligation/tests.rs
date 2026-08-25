//! Round-trip and accessor behaviour for [`PassportObligation`].

use super::*;

#[test]
fn each_variant_round_trips() {
    let cases = [
        PassportObligation::Required {
            from: Some(ObligationDate {
                date: "2027-02-18".to_owned(),
                basis: DateBasis::Sourced,
            }),
        },
        PassportObligation::Required { from: None },
        PassportObligation::NotRequired,
        PassportObligation::DisplacedBy {
            system: "EPREL".to_owned(),
            basis: "ESPR Art. 9(4)(b)".to_owned(),
        },
    ];
    for case in cases {
        let json = serde_json::to_string(&case).expect("serialise");
        assert_eq!(
            serde_json::from_str::<PassportObligation>(&json).expect("deserialise"),
            case,
            "round trip failed for {json}"
        );
    }
}

#[test]
fn an_undated_requirement_omits_the_date_rather_than_nulling_it() {
    let json = serde_json::to_string(&PassportObligation::Required { from: None }).unwrap();
    assert_eq!(json, r#"{"obligation":"required"}"#);
}

#[test]
fn only_required_reports_a_passport_duty() {
    assert!(PassportObligation::Required { from: None }.is_required());
    assert!(!PassportObligation::NotRequired.is_required());
    assert!(
        !PassportObligation::DisplacedBy {
            system: "EPREL".to_owned(),
            basis: "ESPR Art. 9(4)(b)".to_owned(),
        }
        .is_required()
    );
}
