//! The test vectors of Appendix C of draft-ietf-oauth-status-list-21: four lists of
//! 2^20 entries, one for each width, in the text the draft prints.
//!
//! `draft_21_appendix_c.txt` was extracted from the draft by a script and not typed,
//! and says which text it was read from. Each list is read here, and every entry of
//! it must be the value the draft says, or zero.

use super::super::TokenStatusList;

const VECTORS: &str = include_str!("../draft_21_appendix_c.txt");

/// 2^20, the size the draft gives every list of the appendix.
const ENTRIES: u64 = 1 << 20;

struct Vector {
    name: String,
    bits: u8,
    /// The entries the draft lists; every other one is zero.
    statuses: Vec<(u64, u8)>,
    lst: String,
}

fn vectors() -> Vec<Vector> {
    let mut found: Vec<Vector> = Vec::new();
    for line in VECTORS.lines().filter(|line| !line.starts_with('#')) {
        let words: Vec<&str> = line.split(' ').collect();
        match words[..] {
            ["vector", name, "bits", bits] => found.push(Vector {
                name: name.to_owned(),
                bits: bits.parse().expect("a width"),
                statuses: Vec::new(),
                lst: String::new(),
            }),
            ["status", idx, value] => found
                .last_mut()
                .expect("a vector comes first")
                .statuses
                .push((
                    idx.parse().expect("an index"),
                    value.parse().expect("a value"),
                )),
            ["lst", text] => found.last_mut().expect("a vector comes first").lst = text.to_owned(),
            _ => panic!("a line nothing here reads: {line}"),
        }
    }
    found
}

#[test]
fn the_file_holds_the_four_lists_the_draft_prints() {
    let found = vectors();
    let shape: Vec<(&str, u8, usize)> = found
        .iter()
        .map(|v| (v.name.as_str(), v.bits, v.statuses.len()))
        .collect();
    assert_eq!(
        shape,
        [
            ("C.1", 1, 11),
            ("C.2", 2, 11),
            ("C.3", 4, 15),
            ("C.4", 8, 256)
        ]
    );
    assert!(found.iter().all(|v| !v.lst.is_empty()));
}

#[test]
fn each_list_reads_to_the_values_the_draft_prints_and_to_nothing_else() {
    for vector in vectors() {
        let list = TokenStatusList::from_encoded(vector.bits, &vector.lst)
            .unwrap_or_else(|e| panic!("{}: {e}", vector.name));
        assert_eq!(list.entries(), ENTRIES, "{}", vector.name);
        for (idx, value) in &vector.statuses {
            assert_eq!(list.get(*idx), Some(*value), "{} entry {idx}", vector.name);
        }
        // Everything the draft does not list is zero, which is as much a part of
        // the vector as what it lists: a reader that gets another entry wrong
        // shows up here.
        let set = (0..ENTRIES).filter(|idx| list.get(*idx) != Some(0)).count();
        let listed = vector
            .statuses
            .iter()
            .filter(|(_, value)| *value != 0)
            .count();
        assert_eq!(set, listed, "{}", vector.name);
    }
}

/// Writing the entries one by one packs the bytes the draft's list holds, so what
/// `set` does to a neighbour is the same as what the draft's issuer did.
#[test]
fn a_list_built_entry_by_entry_holds_the_drafts_bytes() {
    for vector in vectors() {
        let mut built = TokenStatusList::new(vector.bits, ENTRIES).unwrap();
        for (idx, value) in &vector.statuses {
            built.set(*idx, *value).unwrap();
        }
        let read = TokenStatusList::from_encoded(vector.bits, &vector.lst).unwrap();
        assert_eq!(built, read, "{}", vector.name);
        assert_eq!(
            TokenStatusList::from_encoded(vector.bits, &built.encode().expect("compresses")),
            Ok(read),
            "{} did not survive being encoded",
            vector.name
        );
    }
}
