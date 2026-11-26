//! Property and reference elements.

use super::super::*;

#[test]
fn property_helpers() {
    let s = string_property("name", "cotton", Some("urn:eclass:0173-1#01-AAA000#001"));
    if let AasSubmodelElement::Property(p) = s {
        assert_eq!(p.id_short, "name");
        assert_eq!(p.value, "cotton");
        assert_eq!(
            p.semantic_id.unwrap().keys[0].value,
            "urn:eclass:0173-1#01-AAA000#001"
        );
    }

    let d = double_property("co2e", 8.5, None);
    if let AasSubmodelElement::Property(p) = d {
        assert_eq!(p.value, "8.5");
        assert_eq!(p.value_type, AasDataType::Double);
    }

    let b = boolean_property("available", true, None);
    if let AasSubmodelElement::Property(p) = b {
        assert_eq!(p.value, "true");
        assert_eq!(p.value_type, AasDataType::Boolean);
    }

    let i = integer_property("cycles", 3000, None);
    if let AasSubmodelElement::Property(p) = i {
        assert_eq!(p.value, "3000");
        assert_eq!(p.value_type, AasDataType::Integer);
    }
}

/// A `Property` never carries a `unit` member.
///
/// It is not part of the class — `unit` belongs to
/// `DataSpecificationIec61360`, reached through `embeddedDataSpecifications`.
/// These two tests previously asserted the opposite, which is how the defect
/// survived: the suite agreed with the code instead of with the metamodel.
///
/// Every JSON Schema accepted the bare member (none of 3.0/3.1/3.2 sets
/// `additionalProperties`), and aas-core-works' reference implementation
/// refused the whole document over it.
#[test]
fn a_property_carries_no_unit_member() {
    for prop in [
        double_property("weight", 1.5, None),
        string_property("name", "test", None),
        integer_property("cycles", 3000, None),
        boolean_property("available", true, None),
    ] {
        let json = serde_json::to_value(&prop).unwrap();
        assert!(
            json.get("unit").is_none(),
            "a Property must not serialise a `unit` member: {json}"
        );
    }

    // Not vacuous — the members that *are* part of the class still ship.
    let json = serde_json::to_value(double_property("weight", 1.5, None)).unwrap();
    assert_eq!(json["value"], "1.5");
    assert_eq!(json["valueType"], "xs:double");
    assert_eq!(json["modelType"], "Property");
}

// ── Reference element tests ───────────────────────────────────────────

#[test]
fn reference_element_round_trip() {
    let elem = AasSubmodelElement::ReferenceElement(AasReference::external(
        "repairManualUrl",
        "https://example.com/repair.pdf",
    ));
    let json = serde_json::to_value(&elem).unwrap();
    // `ReferenceElement` is the metamodel class name, and `value` is a
    // `Reference`, not a bare string.
    assert_eq!(json["modelType"], "ReferenceElement");
    assert_eq!(json["idShort"], "repairManualUrl");
    assert_eq!(json["value"]["type"], "ExternalReference");
    assert_eq!(json["value"]["keys"][0]["type"], "GlobalReference");
    assert_eq!(
        json["value"]["keys"][0]["value"],
        "https://example.com/repair.pdf"
    );
    let back: AasSubmodelElement = serde_json::from_value(json).unwrap();
    assert_eq!(elem, back);
}

// ── Shell + submodel builder tests ───────────────────────────────────
