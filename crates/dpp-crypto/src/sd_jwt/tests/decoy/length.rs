//! Decoy digests (RFC 9901 clause 4.2.5): the length of an `_sd` array, and the number
//! of placeholders in an array, does not give the number of claims or elements hidden.
//!
//! The mechanism is `builder::DECOY_BUCKET`: the count rounds up to a multiple of
//! eight. These tests pin what that buys a reader, and that a verifier still reads the
//! token the way it read one without decoys.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::super::super::{ClaimPath, Disclosure, SdJwt, build_payload, conceal, conceal_elements};
use super::super::disclosure::stub_jwt;

/// An object of `members` members, `m0`, `m1`, and so on.
fn object_of(members: usize) -> Map<String, Value> {
    (0..members)
        .map(|i| (format!("m{i}"), json!(i)))
        .collect::<Map<_, _>>()
}

/// The digests of `object`'s `_sd` array.
fn sd_of(object: &Map<String, Value>) -> Vec<String> {
    object["_sd"]
        .as_array()
        .expect("the object carries `_sd`")
        .iter()
        .map(|digest| digest.as_str().expect("a string").to_owned())
        .collect()
}

fn is_placeholder(item: &Value) -> bool {
    item.get("...").is_some()
}

#[test]
fn the_sd_array_is_rounded_up_to_a_multiple_of_eight() {
    for (hidden, carried) in [
        (1, 8),
        (2, 8),
        (7, 8),
        (8, 8),
        (9, 16),
        (15, 16),
        (16, 16),
        (17, 24),
    ] {
        let (kept, disclosures) = conceal(&object_of(hidden), |_| true).unwrap();
        assert_eq!(disclosures.len(), hidden);
        assert_eq!(
            sd_of(&kept).len(),
            carried,
            "{hidden} claims hidden should carry {carried} digests"
        );
    }
}

/// The reader's question: how many claims did this issuer withhold? Two passports that
/// withhold different numbers answer it the same way.
#[test]
fn two_passports_that_withhold_different_numbers_of_claims_carry_arrays_of_one_length() {
    let passport = object_of(12);

    let (few, few_disclosures) = conceal(&passport, |name| ["m0", "m1"].contains(&name)).unwrap();
    let (many, many_disclosures) = conceal(&passport, |name| {
        ["m0", "m1", "m2", "m3", "m4", "m5"].contains(&name)
    })
    .unwrap();

    assert_eq!(few_disclosures.len(), 2);
    assert_eq!(many_disclosures.len(), 6);
    assert_eq!(sd_of(&few).len(), sd_of(&many).len());
}

#[test]
fn an_object_that_hides_nothing_carries_no_sd_array_and_no_decoys() {
    let (kept, disclosures) = conceal(&object_of(5), |_| false).unwrap();
    assert!(disclosures.is_empty());
    assert!(!kept.contains_key("_sd"));
    assert_eq!(kept.len(), 5);
}

#[test]
fn the_disclosures_returned_are_the_real_ones_only() {
    let (kept, disclosures) = conceal(&object_of(6), |name| name < "m3").unwrap();
    assert_eq!(disclosures.len(), 3);

    let digests = sd_of(&kept);
    let held: BTreeSet<String> = disclosures.iter().map(Disclosure::digest).collect();
    assert_eq!(digests.len(), 8);
    assert!(held.iter().all(|digest| digests.contains(digest)));
    assert_eq!(
        digests
            .iter()
            .filter(|digest| !held.contains(*digest))
            .count(),
        5,
        "the other five are decoys, and no Disclosure answers to them"
    );
}

#[test]
fn no_digest_repeats_in_one_array_or_across_two_issuances() {
    let (first, _) = conceal(&object_of(3), |_| true).unwrap();
    let (second, _) = conceal(&object_of(3), |_| true).unwrap();
    let (first, second) = (sd_of(&first), sd_of(&second));

    assert_eq!(first.iter().collect::<BTreeSet<_>>().len(), first.len());
    assert!(
        first.iter().all(|digest| !second.contains(digest)),
        "a digest, a decoy included, repeated across two issuances links them"
    );
}

/// Decoys put after the real digests would be the last of the array, and a reader
/// would know which digests to count. They are put in before the sort, so a digest's
/// place is its value's.
#[test]
fn decoys_are_not_kept_apart_from_the_real_digests() {
    let positions: BTreeSet<usize> = (0..96)
        .map(|_| {
            let (kept, disclosures) = conceal(&object_of(1), |_| true).unwrap();
            let real = disclosures[0].digest();
            sd_of(&kept)
                .iter()
                .position(|digest| *digest == real)
                .expect("the real digest is there")
        })
        .collect();
    assert!(
        positions.len() > 1,
        "the real digest was always at {positions:?}"
    );
}

#[test]
fn an_array_that_hides_an_element_carries_eight_placeholders() {
    let items = [json!("a"), json!("b"), json!("c")];
    let (kept, disclosures) = conceal_elements(&items, |position, _| position == 1);

    assert_eq!(disclosures.len(), 1);
    assert_eq!(
        kept.iter().filter(|item| is_placeholder(item)).count(),
        8,
        "one for the element, seven decoys"
    );
    let in_the_clear: Vec<&Value> = kept.iter().filter(|item| !is_placeholder(item)).collect();
    assert_eq!(
        in_the_clear,
        vec![&json!("a"), &json!("c")],
        "what was not hidden stays, in order"
    );
}

#[test]
fn an_array_that_hides_nothing_gains_nothing() {
    let items = [json!(1), json!(2), json!(3)];
    let (kept, disclosures) = conceal_elements(&items, |_, _| false);
    assert_eq!(kept, items);
    assert!(disclosures.is_empty());
}

/// A decoy that could only be appended, or only follow an element, would show the
/// real placeholders by where they are not.
#[test]
fn decoy_placeholders_land_anywhere_among_the_elements() {
    let items = [json!("hidden"), json!("clear")];
    let mut first = BTreeSet::new();
    let mut last = BTreeSet::new();
    for _ in 0..400 {
        let (kept, disclosures) = conceal_elements(&items, |position, _| position == 0);
        assert_eq!(kept.len(), 9);
        let real = json!({ "...": disclosures[0].digest() });
        // The two elements keep their order, with decoys before, between and after.
        first.insert(kept.iter().position(|item| *item == real).unwrap());
        last.insert(
            kept.iter()
                .position(|item| *item == json!("clear"))
                .unwrap(),
        );
    }
    assert!(first.contains(&0), "a decoy always came first: {first:?}");
    assert!(first.contains(&7), "a decoy never came first: {first:?}");
    assert!(last.contains(&8), "nothing ever came after: {last:?}");
    assert!(last.contains(&1), "a decoy always came last: {last:?}");
}

/// A verifier reads a token with decoys as it read one without, and a Holder reaches an
/// array element by the position the Issuer gave it, whatever stands between.
#[test]
fn a_token_with_decoys_reads_back_and_selects_by_position_as_before() {
    let claims = json!({
        "name": "Alice",
        "age": 30,
        "nat": ["DE", "FR", "IT", "ES"],
    });
    let claims = claims.as_object().unwrap();

    // Repeated, because the decoys fall in new places each time.
    for _ in 0..24 {
        let mut root = claims.clone();
        let nat: Vec<Value> = root["nat"].as_array().unwrap().clone();
        let (kept_nat, mut disclosures) = conceal_elements(&nat, |position, _| position == 1);
        root.insert("nat".to_owned(), Value::Array(kept_nat));
        let (kept, mut root_disclosures) = conceal(&root, |name| name == "age").unwrap();
        disclosures.append(&mut root_disclosures);

        let token = SdJwt::new(stub_jwt(&build_payload(kept, true)), disclosures);

        assert_eq!(
            Value::Object(
                token
                    .disclosed_payload()
                    .expect("a token with decoys reads")
            ),
            Value::Object(claims.clone())
        );

        let wanted = token
            .digests_for_paths(&[ClaimPath::claim("nat").index(1)])
            .expect("position 1 is in the token");
        assert_eq!(
            wanted.len(),
            1,
            "position 1 is `FR`, whatever the decoys do"
        );
        let shown = token
            .present(&wanted.iter().map(String::as_str).collect::<Vec<_>>())
            .expect("presentable")
            .disclosed_payload()
            .expect("processes");
        assert_eq!(shown["nat"], json!(["DE", "FR", "IT", "ES"]));
        assert!(!shown.contains_key("age"));
    }
}
