//! RFC 9901 clause 9.3: salts, and the unlinkability that rests on them.
//!
//! Split from [`super::tests`], which holds the combined-format and parsing
//! half. These are one subject — every test here fails if two credentials for
//! the same field can be recognised as the same field.

use serde_json::{Map, Value, json};

use super::builder::conceal;
use super::disclosure::Disclosure;
use super::tests::{HIDDEN, sample};

/// RFC 9901 clause 4.2.4.1: "The Issuer MUST hide the original order of the
/// claims in the array."
///
/// The failure this guards is silent and easy to write: pushing each digest as
/// the field is walked puts the array in source order, so a reader learns the
/// original structure from a token that discloses nothing.
#[test]
fn digest_order_does_not_follow_claim_order() {
    let (concealed, disclosures) = conceal(&sample(), |name| HIDDEN.contains(&name)).unwrap();

    let published: Vec<&str> = concealed["_sd"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let in_claim_order: Vec<String> = disclosures.iter().map(Disclosure::digest).collect();

    let mut sorted = in_claim_order.clone();
    sorted.sort();
    assert_eq!(published, sorted, "digests must be published sorted");

    // The sorted order must be a genuine reordering for at least one salt draw,
    // otherwise this test would pass on an implementation that does not sort.
    // Salts are random, so a single draw could coincide; repeat until one
    // differs, which is overwhelmingly the first iteration.
    let reordered = (0..32).any(|_| {
        let (c, d) = conceal(&sample(), |name| HIDDEN.contains(&name)).unwrap();
        let pub_order: Vec<String> = c["_sd"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        let claim_order: Vec<String> = d.iter().map(Disclosure::digest).collect();
        pub_order != claim_order
    });
    assert!(reordered, "sorting never reordered — suspect a no-op sort");
}

/// RFC 9901 clause 9.3: a new salt for each claim, 128 bits recommended.
#[test]
fn salts_are_unique_and_128_bits() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let (_, disclosures) = conceal(&sample(), |name| HIDDEN.contains(&name)).unwrap();

    let mut salts: Vec<&str> = disclosures.iter().map(Disclosure::salt).collect();
    let count = salts.len();
    salts.sort_unstable();
    salts.dedup();
    assert_eq!(salts.len(), count, "a salt was reused across claims");

    for d in &disclosures {
        assert_eq!(b64.decode(d.salt()).unwrap().len(), 16);
    }
}

/// Clause 9.3 again, in the direction that matters commercially: two issuances
/// of the same data must not be linkable by their digests.
#[test]
fn two_issuances_of_the_same_claims_share_no_digest() {
    let (first, _) = conceal(&sample(), |name| HIDDEN.contains(&name)).unwrap();
    let (second, _) = conceal(&sample(), |name| HIDDEN.contains(&name)).unwrap();

    let a: Vec<&Value> = first["_sd"].as_array().unwrap().iter().collect();
    let b: Vec<&Value> = second["_sd"].as_array().unwrap().iter().collect();
    assert!(
        a.iter().all(|d| !b.contains(d)),
        "a digest repeated across two issuances — salts are being reused"
    );
}

/// Clause 9.3's same-name-different-place requirement. A `$ref`'d definition
/// carries its disclosure class to every path that references it, so one leaf
/// name legitimately appears at several paths.
#[test]
fn the_same_claim_name_at_two_places_gets_two_salts() {
    let outer = json!({ "serialNumber": "A" }).as_object().unwrap().clone();
    let inner = json!({ "serialNumber": "A" }).as_object().unwrap().clone();

    let (_, a) = conceal(&outer, |n| n == "serialNumber").unwrap();
    let (_, b) = conceal(&inner, |n| n == "serialNumber").unwrap();

    assert_ne!(a[0].salt(), b[0].salt());
    assert_ne!(a[0].digest(), b[0].digest());
}

/// RFC 9901 clause 9.3 as a **property**, not as one sample.
///
/// The three tests above pin uniqueness within a credential, across two
/// issuances, and for one name at two places — each over a single fixed object
/// of five claims. That is the shape of the requirement, checked once. What it
/// cannot see is uniqueness failing at a claim count nobody wrote a fixture for,
/// or a salt repeating on the fourth issuance rather than the second, which is
/// what a correlation leak would actually look like in the field: rare, and
/// invisible to any example chosen in advance.
///
/// So: arbitrary claim sets, issued repeatedly, with every salt ever produced
/// held in one set. A single repeat anywhere fails.
///
/// 🚨 **The detector is [`salts_are_all_new`], and it is proven separately.** A
/// property test over a CSPRNG can only ever observe salts that happen not to
/// repeat, so on its own it is a test whose passing means nothing — it would
/// read identically if its assertion were `true`.
/// [`the_salt_detector_rejects_a_repeat`] feeds the same function a deliberate
/// duplicate. Between them: the detector demonstrably rejects a repeat, and the
/// property runs that detector over everything issuance produces.
#[test]
fn no_salt_ever_repeats_across_claims_or_issuances() {
    use base64::Engine;
    use proptest::prelude::*;
    use std::collections::HashSet;

    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    proptest!(|(
        names in prop::collection::hash_set("[a-z][a-z0-9]{0,12}", 1..12),
        issuances in 2usize..6,
    )| {
        // Prefixed so the generator cannot stumble onto a registered claim
        // name, which `conceal` refuses by design — that refusal is
        // `reserved_claim_names_are_refused`'s to test, not this one's.
        let object: Map<String, Value> = names
            .iter()
            .map(|n| (format!("f_{n}"), json!("v")))
            .collect();

        let mut seen: HashSet<String> = HashSet::new();
        for _ in 0..issuances {
            let (_, disclosures) = conceal(&object, |_| true).unwrap();
            prop_assert_eq!(disclosures.len(), object.len());
            for d in &disclosures {
                prop_assert_eq!(
                    b64.decode(d.salt()).unwrap().len(),
                    16,
                    "salt is not 128 bits"
                );
            }
            prop_assert!(
                salts_are_all_new(&disclosures, &mut seen),
                "a salt repeated — two credentials for the same field are linkable"
            );
        }
    });
}

/// Records every salt in `disclosures` and answers whether all of them were new.
///
/// Extracted so the property above and the negative test below run the *same*
/// check, rather than the negative test proving something the property does not
/// use.
fn salts_are_all_new(
    disclosures: &[Disclosure],
    seen: &mut std::collections::HashSet<String>,
) -> bool {
    // A loop rather than `.all(...)`, which short-circuits: every salt is
    // recorded even after a repeat is found, because returning early would
    // leave the rest unseen and a later call would then miss a duplicate of
    // one of them.
    let mut all_new = true;
    for d in disclosures {
        all_new &= seen.insert(d.salt().to_owned());
    }
    all_new
}

/// The detector must reject a repeat, or the property test above is decoration.
///
/// Deterministic, and it does not reach for the CSPRNG. Stubbing `os_rng` would
/// mean cutting a seam into production issuance whose only caller is a test —
/// and `Disclosure::new` guaranteeing the clause 9.3 property *because it has no
/// other salt source* is the reason that guarantee holds. `with_salt` exists for
/// exactly this: a caller-supplied salt, so a duplicate can be built on purpose
/// and handed to the check that has to catch it.
#[test]
fn the_salt_detector_rejects_a_repeat() {
    let repeated = "AAAAAAAAAAAAAAAAAAAAAA".to_owned();
    let mut seen = std::collections::HashSet::new();

    let first = [Disclosure::with_salt(repeated.clone(), "a", json!(1)).unwrap()];
    assert!(
        salts_are_all_new(&first, &mut seen),
        "the first sighting of a salt is not a repeat"
    );

    // The same salt on a different claim, which is the correlation leak: two
    // credentials whose disclosures for one field are byte-identical.
    let second = [Disclosure::with_salt(repeated, "b", json!(2)).unwrap()];
    assert!(
        !salts_are_all_new(&second, &mut seen),
        "a repeated salt was accepted — the property test above would pass \
         through a broken CSPRNG"
    );

    // And a fresh one still passes, so the check is not simply always false
    // after its first call.
    let third =
        [Disclosure::with_salt("BBBBBBBBBBBBBBBBBBBBBB".to_owned(), "c", json!(3)).unwrap()];
    assert!(salts_are_all_new(&third, &mut seen));
}
