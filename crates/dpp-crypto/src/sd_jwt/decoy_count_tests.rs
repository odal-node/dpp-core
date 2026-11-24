//! `conceal_to`: a caller that knows how many members an object could hide states it, and
//! the `_sd` array is that long whatever the object hides.
//!
//! `decoy_tests` pins the bucket a caller that states no count gets. These pin what a
//! count buys over it: no bucket step, and an object that hides nothing still looks like
//! one that could.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::tests::stub_jwt;
use super::{SdJwt, build_payload, conceal, conceal_to};

/// An object of `members` members, `m0`, `m1`, and so on.
fn object_of(members: usize) -> Map<String, Value> {
    (0..members)
        .map(|i| (format!("m{i}"), json!(i)))
        .collect::<Map<_, _>>()
}

fn sd_len(object: &Map<String, Value>) -> Option<usize> {
    object.get("_sd").and_then(Value::as_array).map(Vec::len)
}

/// Hides the first `hidden` members of an object of twelve, and states that it could
/// hide ten.
fn hiding(hidden: usize) -> (Option<usize>, usize) {
    let names: BTreeSet<String> = (0..hidden).map(|i| format!("m{i}")).collect();
    let (kept, disclosures) = conceal_to(&object_of(12), |name| names.contains(name), 10).unwrap();
    (sd_len(&kept), disclosures.len())
}

#[test]
fn a_stated_count_is_the_length_whatever_the_object_hides() {
    for hidden in [0, 1, 2, 7, 8, 9, 10] {
        let (length, disclosures) = hiding(hidden);
        assert_eq!(length, Some(10), "{hidden} hidden of a possible ten");
        assert_eq!(disclosures, hidden);
    }
}

/// Where a bucket of eight would step from eight to sixteen, a stated count does not.
#[test]
fn a_stated_count_has_no_bucket_step() {
    assert_eq!(hiding(8).0, hiding(9).0);
    let nine: BTreeSet<String> = (0..9).map(|i| format!("m{i}")).collect();
    let (kept, _) = conceal(&object_of(12), |name| nine.contains(name)).unwrap();
    assert_eq!(
        sd_len(&kept),
        Some(16),
        "the bucket does step, at the ninth"
    );
}

/// An object that could hide something and hides nothing carries decoys alone, and
/// returns no disclosures: it looks like one that did.
#[test]
fn an_object_that_hides_nothing_still_carries_the_stated_count_of_decoys() {
    let (kept, disclosures) = conceal_to(&object_of(3), |_| false, 5).unwrap();
    assert!(disclosures.is_empty());
    assert_eq!(sd_len(&kept), Some(5));
    assert_eq!(
        kept.len(),
        3 + 1,
        "the three members in the clear, and `_sd`"
    );
}

/// A count below the number hidden adds nothing, and the token is still valid: it shows
/// the exact number, as an object with no count would.
#[test]
fn a_count_below_the_number_hidden_adds_no_decoys() {
    let (kept, disclosures) = conceal_to(&object_of(6), |_| true, 4).unwrap();
    assert_eq!(disclosures.len(), 6);
    assert_eq!(sd_len(&kept), Some(6));
}

/// A count of zero is `conceal`: no `_sd` when nothing is hidden, the bucket otherwise.
#[test]
fn a_count_of_zero_is_the_bucket_and_no_sd_when_nothing_is_hidden() {
    let (kept, _) = conceal_to(&object_of(3), |_| false, 0).unwrap();
    assert_eq!(sd_len(&kept), None);
    let (kept, _) = conceal_to(&object_of(3), |name| name == "m0", 0).unwrap();
    assert_eq!(sd_len(&kept), Some(8));
}

/// Decoys alone read back as an object with nothing hidden: the reader removes `_sd`.
#[test]
fn a_token_whose_decoys_hide_nothing_reads_back_as_it_was() {
    let claims = json!({ "name": "Alice", "age": 30 });
    let claims = claims.as_object().unwrap();
    let (kept, disclosures) = conceal_to(claims, |_| false, 4).unwrap();
    assert_eq!(sd_len(&kept), Some(4));

    let token = SdJwt::new(stub_jwt(&build_payload(kept, true)), disclosures);
    let read = token.disclosed_payload().expect("a token of decoys reads");
    assert_eq!(&read, claims);
}
