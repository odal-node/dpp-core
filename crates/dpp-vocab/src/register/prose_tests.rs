//! The counts and lists the crate's prose states are derived from the register.

use std::path::Path;

use super::*;
use crate::vocabulary::Status;

fn register() -> VocabularyRegister {
    VocabularyRegister::new()
}

/// A count in words, as the prose writes it ("Six", "twenty-one").
fn in_words(n: usize, capital: bool) -> String {
    const ONES: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 8] = [
        "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    assert!(n < 100, "extend in_words before the register passes 99");
    let word = match n {
        0..=19 => ONES[n].to_owned(),
        _ if n.is_multiple_of(10) => TENS[n / 10 - 2].to_owned(),
        _ => format!("{}-{}", TENS[n / 10 - 2], ONES[n % 10]),
    };
    if capital {
        let mut chars = word.chars();
        chars
            .next()
            .map(|c| c.to_uppercase().chain(chars).collect())
            .unwrap_or_default()
    } else {
        word
    }
}

/// The file's text with comment markers and line breaks removed, so a sentence
/// wrapped across lines can be searched for as written.
fn flattened(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} is unreadable: {e}", path.display()))
        .lines()
        .map(|line| line.trim_start().trim_start_matches("//!").trim())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn code_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A figure stated in prose is a copy of the register, and a copy is what drifts:
/// this crate's prose once said how many records were re-checked and which kinds of
/// change they carried, in a paragraph nobody was going to re-read when a record
/// changed. These sentences are built from the register and searched for, so the
/// next record to become `verified`, or term to be read, fails the build until the
/// prose is updated.
#[test]
fn the_counts_and_lists_in_the_prose_come_from_the_register() {
    let register = register();
    let total = register.all().len();
    let mut verified: Vec<String> = register
        .all()
        .iter()
        .filter(|v| v.status == Status::Verified)
        .map(|v| v.key.clone())
        .collect();
    verified.sort_unstable();
    let mut terms: Vec<String> = register
        .all()
        .iter()
        .flat_map(|v| v.terms.iter().map(|t| t.iri.clone()))
        .collect();
    terms.sort_unstable();

    let state = format!(
        "{} of the {} records in `vocabularies/` are verified ({}):",
        in_words(verified.len(), true),
        in_words(total, false),
        code_list(&verified)
    );
    let read = if terms.is_empty() {
        "No term has been read so far.".to_owned()
    } else {
        format!("Terms read so far: {}.", code_list(&terms))
    };
    let directory = format!(
        "{} of the {} records are `verified`;",
        in_words(verified.len(), true),
        in_words(total, false)
    );

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut missing = Vec::new();
    for (file, sentences) in [
        ("README.md", vec![&state, &read]),
        ("src/lib.rs", vec![&state, &read]),
        ("vocabularies/README.md", vec![&directory]),
    ] {
        let text = flattened(&root.join(file));
        for sentence in sentences {
            if !text.contains(sentence.as_str()) {
                missing.push(format!("{file} does not contain: {sentence}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "prose that restates the register has drifted from it:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn counts_are_written_in_words_the_way_the_prose_writes_them() {
    assert_eq!(in_words(6, true), "Six");
    assert_eq!(in_words(20, false), "twenty");
    assert_eq!(in_words(21, false), "twenty-one");
    assert_eq!(in_words(0, true), "Zero");
}
