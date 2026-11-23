//! The reference a credential carries: section 6.2.

use serde_json::{Map, Value, json};

use super::{StatusReference, TokenStatusListError};

fn claims(value: &Value) -> Map<String, Value> {
    value.as_object().expect("a JSON object").clone()
}

/// The payload of the draft's example of a Referenced Token (section 6.2).
fn drafts_example() -> Value {
    json!({ "status": { "status_list": { "idx": 0, "uri": "https://example.com/statuslists/1" } } })
}

#[test]
fn the_claim_is_the_one_the_draft_shows() {
    let reference = StatusReference {
        idx: 0,
        uri: "https://example.com/statuslists/1".to_owned(),
    };
    assert_eq!(json!({ "status": reference.to_claim() }), drafts_example());
}

#[test]
fn the_drafts_example_is_read() {
    assert_eq!(
        StatusReference::from_claims(&claims(&drafts_example())),
        Ok(Some(StatusReference {
            idx: 0,
            uri: "https://example.com/statuslists/1".to_owned(),
        }))
    );
}

#[test]
fn a_reference_survives_being_written_and_read() {
    let reference = StatusReference {
        idx: u64::MAX,
        uri: "https://status.example/lists/7".to_owned(),
    };
    let written = claims(&json!({ "status": reference.to_claim() }));
    assert_eq!(StatusReference::from_claims(&written), Ok(Some(reference)));
}

#[test]
fn no_reference_is_not_an_error() {
    assert_eq!(StatusReference::from_claims(&claims(&json!({}))), Ok(None));
    // Another status mechanism beside, or instead of, a list.
    let other = json!({ "status": { "other_mechanism": { "id": 7 } } });
    assert_eq!(StatusReference::from_claims(&claims(&other)), Ok(None));
    let both = json!({
        "status": {
            "other_mechanism": { "id": 7 },
            "status_list": { "idx": 3, "uri": "https://status.example/1" },
        }
    });
    assert_eq!(
        StatusReference::from_claims(&claims(&both)),
        Ok(Some(StatusReference {
            idx: 3,
            uri: "https://status.example/1".to_owned(),
        }))
    );
}

/// An unreadable reference is a credential nobody can withdraw, and reading it as
/// an absent one would hide that.
#[test]
fn a_reference_that_cannot_be_read_is_refused() {
    let uri = "https://status.example/1";
    let cases = [
        (json!({ "status": "draft" }), "status"),
        (json!({ "status": 7 }), "status"),
        (json!({ "status": null }), "status"),
        (json!({ "status": { "status_list": "x" } }), "status_list"),
        (json!({ "status": { "status_list": null } }), "status_list"),
        (
            json!({ "status": { "status_list": { "uri": uri } } }),
            "idx",
        ),
        (
            json!({ "status": { "status_list": { "idx": -1, "uri": uri } } }),
            "idx",
        ),
        (
            json!({ "status": { "status_list": { "idx": "0", "uri": uri } } }),
            "idx",
        ),
        (
            json!({ "status": { "status_list": { "idx": 1.5, "uri": uri } } }),
            "idx",
        ),
        (json!({ "status": { "status_list": { "idx": 0 } } }), "uri"),
        (
            json!({ "status": { "status_list": { "idx": 0, "uri": 7 } } }),
            "uri",
        ),
        (
            json!({ "status": { "status_list": { "idx": 0, "uri": "" } } }),
            "uri",
        ),
    ];
    for (payload, member) in cases {
        assert_eq!(
            StatusReference::from_claims(&claims(&payload)),
            Err(TokenStatusListError::MalformedReference(member)),
            "{payload}"
        );
    }
}
