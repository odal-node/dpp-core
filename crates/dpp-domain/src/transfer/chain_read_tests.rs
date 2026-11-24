//! [`TransferChain::from_value`]: a refused chain and a malformed document come back as different variants.

use serde_json::json;

use super::chain_tests::{append_completed, append_pending, operator, refused, to_json};
use super::{ChainReadError, TransferChain, TransferError};
use crate::passport::PassportId;

/// A chain with one completed record, and the same chain as stored JSON.
fn one_completed_record() -> (TransferChain, serde_json::Value) {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a);
    append_completed(&mut chain, &b);
    let value = to_json(&chain);
    (chain, value)
}

#[test]
fn a_chain_that_breaks_the_invariant_is_refused_with_the_passport_and_the_record() {
    let (chain, mut value) = one_completed_record();
    let transfer_id = chain.records()[0].transfer_id;
    let other = PassportId::new();
    value["transfers"][0]["passportId"] = json!(other);

    let error = TransferChain::from_value(&value).expect_err("the chain breaks the invariant");

    let ChainReadError::Refused(TransferError::InvalidChain {
        passport_id,
        index,
        transfer_id: cited,
        reason,
    }) = error
    else {
        panic!("a chain that breaks the invariant is Refused(InvalidChain), got {error:?}");
    };
    assert_eq!(passport_id, chain.passport_id().to_string());
    assert_eq!(index, 0);
    assert_eq!(cited, transfer_id.to_string());
    assert_eq!(
        *reason,
        TransferError::PassportMismatch {
            expected: chain.passport_id().to_string(),
            got: other.to_string(),
        }
    );
}

#[test]
fn a_document_that_is_not_a_chain_is_malformed_not_refused() {
    let (_, mut value) = one_completed_record();
    value.as_object_mut().unwrap().remove("transfers");

    let error = TransferChain::from_value(&value).expect_err("a chain needs its transfers");

    assert!(
        matches!(error, ChainReadError::Malformed(_)),
        "a missing member is Malformed, got {error:?}"
    );
}

#[test]
fn a_record_of_the_wrong_type_is_malformed_not_refused() {
    let (_, mut value) = one_completed_record();
    value["transfers"][0]["passportId"] = json!(42);

    let error = TransferChain::from_value(&value).expect_err("a passport id is not a number");

    assert!(
        matches!(error, ChainReadError::Malformed(_)),
        "a member of the wrong type is Malformed, got {error:?}"
    );
}

#[test]
fn a_refusal_is_located_at_the_record_that_breaks_the_invariant() {
    let pid = PassportId::new();
    let (a, b, c) = (
        operator("did:web:a.example"),
        operator("did:web:b.example"),
        operator("did:web:c.example"),
    );
    let mut chain = TransferChain::new(pid, a);
    append_completed(&mut chain, &b);
    append_pending(&mut chain, &c);
    let mut value = to_json(&chain);
    value["transfers"][1]["fromOperator"]["did"] = json!("did:web:elsewhere.example");

    let error = TransferChain::from_value(&value).expect_err("the sequence is broken");

    let ChainReadError::Refused(TransferError::InvalidChain { index, reason, .. }) = error else {
        panic!("expected a refusal, got {error:?}");
    };
    assert_eq!(index, 1);
    assert!(matches!(*reason, TransferError::OperatorMismatch { .. }));
}

#[test]
fn a_valid_chain_reads_the_same_as_it_does_through_deserialize() {
    let (chain, value) = one_completed_record();

    let read = TransferChain::from_value(&value).expect("a valid chain must read");
    let deserialized: TransferChain = serde_json::from_value(value).unwrap();

    assert_eq!(to_json(&read), to_json(&chain));
    assert_eq!(to_json(&read), to_json(&deserialized));
}

#[test]
fn deserialize_still_refuses_the_chain_from_value_refuses() {
    let (_, mut value) = one_completed_record();
    value["transfers"][0]["passportId"] = json!(PassportId::new());

    let from_value = TransferChain::from_value(&value).expect_err("refused");
    let message = refused(value);

    assert!(matches!(from_value, ChainReadError::Refused(_)));
    assert_eq!(
        from_value.to_string(),
        message,
        "the refusal reads the same either way"
    );
}
