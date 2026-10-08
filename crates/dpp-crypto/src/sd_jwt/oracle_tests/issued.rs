//! The tokens this crate issues: structures to hide, and every way of presenting
//! each.
//!
//! Each case is a claim tree and a set of paths to hide. The corpus holds the
//! token with everything disclosed, and a token for each reachable presentation
//! of it, with what a verifier must see worked out by [`super::plan::reveal`]
//! and not by this crate.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::plan::{Path, Rng, Seg, path, paths};

pub(super) struct Case {
    pub(super) id: String,
    pub(super) claims: Value,
    pub(super) hidden: BTreeSet<Path>,
}

fn case(id: &str, claims: Value, hidden: &[&[&str]]) -> Case {
    Case {
        id: id.to_owned(),
        claims,
        hidden: paths(hidden),
    }
}

/// How a path reads in a case id or a failure: `address.region`, `nat.#1`.
pub(super) fn show(p: &Path) -> String {
    p.iter()
        .map(|s| match s {
            Seg::Key(k) => k.clone(),
            Seg::Idx(i) => format!("#{i}"),
        })
        .collect::<Vec<_>>()
        .join(".")
}

pub(super) fn cases() -> Vec<Case> {
    let mut out = handwritten();
    out.extend((0..24).map(random));
    out
}

fn handwritten() -> Vec<Case> {
    let address = json!({
        "street_address": "Schulstr. 12",
        "locality": "Schulpforta",
        "region": "Sachsen-Anhalt",
        "country": "DE"
    });
    vec![
        case(
            "flat/one-hidden",
            json!({"sub": "user_42", "given_name": "John", "family_name": "Doe"}),
            &[&["given_name"]],
        ),
        case(
            "flat/all-hidden",
            json!({"sub": "user_42", "given_name": "John", "family_name": "Doe"}),
            &[&["sub"], &["given_name"], &["family_name"]],
        ),
        case(
            "flat/none-hidden",
            json!({"sub": "user_42", "given_name": "John"}),
            &[],
        ),
        case(
            "flat/every-json-type",
            json!({
                "s": "text", "i": 42, "neg": -7, "f": 87.5, "yes": true, "no": false,
                "n": null, "o": {}, "a": [], "nested": {"k": "v"}, "arr": [1, "two", null]
            }),
            &[
                &["s"],
                &["i"],
                &["neg"],
                &["f"],
                &["yes"],
                &["no"],
                &["n"],
                &["o"],
                &["a"],
                &["nested"],
                &["arr"],
            ],
        ),
        case(
            "flat/text-edge-cases",
            json!({
                "naïve": "café", "漢字": "日本語", "emoji": "🔐 key", "quote": "say \"hi\"",
                "back": "a\\b", "ctrl": "line1\nline2\ttab", "empty": "",
                "space name": " lead and trail ", "slash/and~tilde": "a/b~c", "": "empty name"
            }),
            &[
                &["naïve"],
                &["漢字"],
                &["emoji"],
                &["quote"],
                &["back"],
                &["ctrl"],
                &["empty"],
                &["space name"],
                &["slash/and~tilde"],
                &[""],
            ],
        ),
        case(
            "nested/object-members",
            json!({"sub": "u", "address": address.clone()}),
            &[
                &["address", "street_address"],
                &["address", "locality"],
                &["address", "region"],
            ],
        ),
        case(
            "nested/object-hidden-whole",
            json!({"sub": "u", "address": address.clone()}),
            &[&["address"]],
        ),
        case(
            "nested/recursive",
            json!({"sub": "u", "address": address}),
            &[
                &["address"],
                &["address", "street_address"],
                &["address", "region"],
            ],
        ),
        case(
            "nested/deep-five",
            json!({"a": {"b": {"c": {"d": {"e": "leaf", "e2": "x"}, "d2": "y"}, "c2": "z"}, "b2": "w"}, "a2": "v"}),
            &[
                &["a"],
                &["a", "b"],
                &["a", "b", "c"],
                &["a", "b", "c", "d"],
                &["a", "b", "c", "d", "e"],
                &["a2"],
            ],
        ),
        case(
            "array/first",
            json!({"nat": ["DE", "FR", "IT"]}),
            &[&["nat", "#0"]],
        ),
        case(
            "array/middle",
            json!({"nat": ["DE", "FR", "IT"]}),
            &[&["nat", "#1"]],
        ),
        case(
            "array/last",
            json!({"nat": ["DE", "FR", "IT"]}),
            &[&["nat", "#2"]],
        ),
        case(
            "array/all",
            json!({"nat": ["DE", "FR", "IT"]}),
            &[&["nat", "#0"], &["nat", "#1"], &["nat", "#2"]],
        ),
        case("array/none", json!({"nat": ["DE", "FR", "IT"]}), &[]),
        case(
            "array/of-objects",
            json!({"records": [{"id": 1, "secret": "a"}, {"id": 2, "secret": "b"}]}),
            &[&["records", "#0", "secret"], &["records", "#1", "secret"]],
        ),
        case(
            "array/element-and-its-members",
            json!({"records": [{"id": 1, "secret": "a"}, {"id": 2, "secret": "b"}]}),
            &[&["records", "#1"], &["records", "#1", "secret"]],
        ),
        case(
            "array/nested-arrays",
            json!({"m": [[1, 2], [3, 4, 5]]}),
            &[&["m", "#0", "#1"], &["m", "#1"]],
        ),
        case(
            "array/mixed-types",
            json!({"v": [1, "two", null, true, {"k": "v"}, [9], "", 2.5]}),
            &[&["v", "#2"], &["v", "#4"], &["v", "#5"], &["v", "#6"]],
        ),
        case(
            "array/empty-and-singleton",
            json!({"e": [], "one": ["x"]}),
            &[&["one", "#0"]],
        ),
        case(
            "array/array-in-hidden-property",
            json!({"tags": ["a", "b", "c"]}),
            &[&["tags"], &["tags", "#1"]],
        ),
        case(
            "mixed/passport-shaped",
            json!({
                "id": "018f3a4c-0000-7000-8000-000000000001",
                "productGroup": "battery",
                "schemaVersion": "2.8.0",
                "manufacturer": {"name": "ACME", "address": {"city": "Gent", "country": "BE"}},
                "productGroupData": {
                    "batteryChemistry": "LFP",
                    "cathodeMaterial": ["LiFePO4", "Mn"],
                    "stateOfHealthPct": 87.5,
                    "testReportResults": {"report": "r-1", "values": [1.5, 2.5]},
                    "safetyMeasures": ["Do not puncture", "Keep dry"]
                }
            }),
            &[
                &["manufacturer", "address"],
                &["productGroupData", "cathodeMaterial", "#1"],
                &["productGroupData", "stateOfHealthPct"],
                &["productGroupData", "testReportResults"],
                &["productGroupData", "testReportResults", "values", "#0"],
                &["productGroupData", "safetyMeasures"],
            ],
        ),
        wide(),
        long_value(),
    ]
}

/// 120 claims, every third hidden: a token with a long `_sd` array.
fn wide() -> Case {
    let mut claims = Map::new();
    let mut hidden = BTreeSet::new();
    for n in 0..120 {
        let name = format!("c{n:03}");
        claims.insert(name.clone(), json!(n));
        if n % 3 == 0 {
            hidden.insert(path(&[&name]));
        }
    }
    Case {
        id: "wide/many-claims".to_owned(),
        claims: Value::Object(claims),
        hidden,
    }
}

/// One claim of 50,000 characters, hidden: a Disclosure far larger than the rest.
fn long_value() -> Case {
    Case {
        id: "wide/long-value".to_owned(),
        claims: json!({"small": 1, "big": "x".repeat(50_000)}),
        hidden: paths(&[&["big"]]),
    }
}

// ----- seeded random trees ---------------------------------------------------

fn random(seed: u64) -> Case {
    let mut rng = Rng(0x5D_5D_0000 + seed);
    let mut root = Map::new();
    for n in 0..(3 + rng.below(5)) {
        root.insert(format!("k{n}"), random_value(&mut rng, 3));
    }
    let claims = Value::Object(root);
    let mut every = Vec::new();
    collect_paths(&claims, &mut Path::new(), &mut every);
    let hidden = every.into_iter().filter(|_| rng.chance(35)).collect();
    Case {
        id: format!("random/seed-{seed:02}"),
        claims,
        hidden,
    }
}

/// Numbers are integers or odd multiples of a quarter. The signer canonicalises
/// numbers (RFC 8785) and prints an integral float without its `.0`, so a float
/// that happened to be whole would read back as an integer and compare unequal
/// as a `serde_json::Value` while meaning the same number. An odd quarter is
/// never whole.
fn random_value(rng: &mut Rng, depth: u32) -> Value {
    let kinds = if depth == 0 { 6 } else { 9 };
    match rng.below(kinds) {
        0 => json!(format!("s{}", rng.below(1000))),
        1 => json!(rng.below(2000) as i64 - 1000),
        2 => json!((2 * rng.below(200) + 1) as f64 / 4.0),
        3 => json!(rng.chance(50)),
        4 => Value::Null,
        5 => json!("café ☕"),
        6 | 7 => {
            let mut map = Map::new();
            for n in 0..(1 + rng.below(4)) {
                map.insert(format!("m{n}"), random_value(rng, depth - 1));
            }
            Value::Object(map)
        }
        _ => Value::Array(
            (0..rng.below(5))
                .map(|_| random_value(rng, depth - 1))
                .collect(),
        ),
    }
}

fn collect_paths(value: &Value, at: &mut Path, out: &mut Vec<Path>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                at.push(Seg::Key(k.clone()));
                out.push(at.clone());
                collect_paths(v, at, out);
                at.pop();
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                at.push(Seg::Idx(i));
                out.push(at.clone());
                collect_paths(v, at, out);
                at.pop();
            }
        }
        _ => {}
    }
}
