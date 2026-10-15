//! A string literal must not carry a run of spaces where a line continuation
//! used to be.
//!
//! A `\` at the end of a line inside a string literal tells Rust to drop the
//! newline and the next line's leading whitespace. Delete the backslash and the
//! newline but keep the indentation, and the literal still compiles: it now
//! carries a run of spaces in the middle of a sentence, and nothing else
//! notices. Seven literals had exactly that, four of them in code this
//! workspace ships: a plugin finding, a public constant, an error message and
//! a panic message.
//!
//! The scan is lexical and deliberately narrow. Inside an ordinary string
//! literal, a run of [`MIN_RUN`] or more spaces between two non-space characters
//! on one source line is reported. Raw strings are left alone, since nothing in
//! them is a continuation. So is a literal that writes an explicit `\n`: aligning
//! the columns of lines it breaks itself is layout, not a lost continuation.

use std::fs;
use std::path::{Path, PathBuf};

/// The shortest run of spaces reported.
///
/// A lost continuation leaves the next line's indentation behind, which is at
/// least four spaces anywhere but column zero. Four is also as low as this goes
/// without reporting something deliberate: at three, a test fixture for a padded
/// label starts to match.
const MIN_RUN: usize = 4;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Every directory of Rust source that ships or tests what ships: each crate's
/// `src` and `tests`, and each plugin's `src`.
///
/// Discovered rather than listed, so a new crate is scanned without anyone
/// remembering to add it.
fn scanned_dirs() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut dirs = Vec::new();
    for (group, subdirs) in [("crates", &["src", "tests"][..]), ("plugins", &["src"][..])] {
        let Ok(entries) = fs::read_dir(root.join(group)) else {
            continue;
        };
        for entry in entries.flatten() {
            for sub in subdirs {
                let dir = entry.path().join(sub);
                if dir.is_dir() {
                    dirs.push(dir);
                }
            }
        }
    }
    dirs
}

fn find_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Every lost continuation in `src`, as 1-based line numbers with an excerpt.
///
/// A small lexer: enough of Rust's token grammar to know when it is inside an
/// ordinary string literal, and nothing more. Comments, raw strings and char
/// literals are skipped so that a quote inside one of them does not flip it.
fn lost_continuations(src: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = src.chars().collect();
    let mut found = Vec::new();
    let mut line = 1;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let prev_is_ident = i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');

        if c == '\n' {
            line += 1;
            i += 1;
        } else if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i = skip_block_comment(&chars, i, &mut line);
        } else if !prev_is_ident && (c == 'r' || (c == 'b' && next == Some('r'))) {
            let start = if c == 'b' { i + 2 } else { i + 1 };
            match raw_string_end(&chars, start, &mut line) {
                Some(end) => i = end,
                None => i += 1,
            }
        } else if c == '\'' {
            i = skip_char_literal_or_lifetime(&chars, i);
        } else if c == '"' {
            let (end, runs) = scan_string(&chars, i + 1, &mut line);
            found.extend(runs);
            i = end;
        } else {
            i += 1;
        }
    }
    found
}

/// Skip a `/* … */` comment, nested ones included. Returns the index after it.
fn skip_block_comment(chars: &[char], start: usize, line: &mut usize) -> usize {
    let mut depth = 0;
    let mut i = start;
    while i < chars.len() {
        match (chars[i], chars.get(i + 1).copied()) {
            ('/', Some('*')) => {
                depth += 1;
                i += 2;
            }
            ('*', Some('/')) => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    return i;
                }
            }
            ('\n', _) => {
                *line += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    i
}

/// If a raw string opens at `start` (after its `r` or `br`), the index after
/// its closing delimiter. `None` when `r` was only the start of an identifier.
fn raw_string_end(chars: &[char], start: usize, line: &mut usize) -> Option<usize> {
    let mut i = start;
    let mut hashes = 0;
    while chars.get(i) == Some(&'#') {
        hashes += 1;
        i += 1;
    }
    if chars.get(i) != Some(&'"') {
        return None;
    }
    i += 1;
    while i < chars.len() {
        if chars[i] == '\n' {
            *line += 1;
        }
        if chars[i] == '"' && (1..=hashes).all(|k| chars.get(i + k) == Some(&'#')) {
            return Some(i + 1 + hashes);
        }
        i += 1;
    }
    Some(i)
}

/// Skip a char literal (`'x'`, `'\''`, `'\u{2014}'`), or step past the quote
/// of a lifetime (`'a`). Returns the index to resume at.
fn skip_char_literal_or_lifetime(chars: &[char], start: usize) -> usize {
    match (chars.get(start + 1), chars.get(start + 2)) {
        (Some('\\'), _) => {
            let mut i = start + 3;
            while i < chars.len() && chars[i] != '\'' {
                i += 1;
            }
            i + 1
        }
        (Some(_), Some('\'')) => start + 3,
        _ => start + 1,
    }
}

/// Scan an ordinary string literal whose content starts at `start`. Returns the
/// index after the closing quote and the runs it reports.
fn scan_string(chars: &[char], start: usize, line: &mut usize) -> (usize, Vec<(usize, String)>) {
    let mut runs = Vec::new();
    let mut writes_newline = false;
    // The literal's text on the current source line, for the excerpt and for
    // telling a run in mid-sentence from one that starts the line.
    let mut on_line = String::new();
    let mut i = start;

    while i < chars.len() {
        match chars[i] {
            '"' => {
                i += 1;
                break;
            }
            '\\' => match chars.get(i + 1) {
                // A continuation: Rust drops the newline and the indentation
                // that follows, so none of it reaches the string.
                Some('\n') | Some('\r') => {
                    i += 1;
                    while i < chars.len() && chars[i].is_whitespace() {
                        if chars[i] == '\n' {
                            *line += 1;
                        }
                        i += 1;
                    }
                    on_line.clear();
                }
                Some('n') => {
                    writes_newline = true;
                    on_line.push_str("\\n");
                    i += 2;
                }
                Some(&other) => {
                    on_line.push('\\');
                    on_line.push(other);
                    i += 2;
                }
                None => i += 1,
            },
            '\n' => {
                *line += 1;
                on_line.clear();
                i += 1;
            }
            ' ' => {
                let run_start = i;
                while chars.get(i) == Some(&' ') {
                    i += 1;
                }
                let run = i - run_start;
                let after = chars.get(i).copied();
                let mid_sentence = !on_line.trim().is_empty()
                    && after.is_some_and(|a| !a.is_whitespace() && a != '"');
                if run >= MIN_RUN && mid_sentence {
                    let tail: String = chars[i..]
                        .iter()
                        .take(24)
                        .take_while(|c| **c != '\n')
                        .collect();
                    runs.push((
                        *line,
                        format!("{}<{run} spaces>{tail}", last_chars(&on_line, 24)),
                    ));
                }
                on_line.extend(std::iter::repeat_n(' ', run));
            }
            other => {
                on_line.push(other);
                i += 1;
            }
        }
    }

    if writes_newline {
        runs.clear();
    }
    (i, runs)
}

fn last_chars(s: &str, n: usize) -> String {
    let count = s.chars().count();
    s.chars().skip(count.saturating_sub(n)).collect()
}

#[test]
fn no_string_literal_carries_a_lost_continuation() {
    let mut files = Vec::new();
    for dir in scanned_dirs() {
        find_rs_files(&dir, &mut files);
    }
    assert!(
        files.len() > 100,
        "found only {} Rust files to scan — has the repo layout moved?",
        files.len()
    );

    let root = workspace_root();
    let mut offenders = Vec::new();
    for path in &files {
        let src =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for (line, excerpt) in lost_continuations(&src) {
            offenders.push(format!("  {rel}:{line}: …{excerpt}…"));
        }
    }

    assert!(
        offenders.is_empty(),
        "\nthese string literals carry a run of spaces mid-sentence, which is what a \
         line continuation leaves behind when its `\\` and newline are deleted:\n{}\n\n\
         End the line with a space and `\\`, and start the next with the rest of the \
         sentence.\n",
        offenders.join("\n")
    );
}

// The scanner has to be shown to fail, or a lexer that silently stopped
// matching would pass this file while checking nothing. Fixtures build their
// runs with `repeat` so that this file's own source carries none.

fn spaces(n: usize) -> String {
    " ".repeat(n)
}

#[test]
fn a_run_where_a_continuation_was_is_reported() {
    let src = format!(
        "fn f() {{\n    let m = \"declared without a year.{}EU 2023/1542 Art. 8(1)\";\n}}\n",
        spaces(18)
    );
    let found = lost_continuations(&src);
    assert_eq!(
        found.len(),
        1,
        "{:?}",
        found.iter().map(|f| &f.1).collect::<Vec<_>>()
    );
    assert_eq!(found[0].0, 2, "reported on the line the literal sits on");
}

#[test]
fn a_continuation_that_kept_its_backslash_is_not_reported() {
    let src = format!(
        "let m = \"declared without a year. \\\n{}EU 2023/1542 Art. 8(1)\";\n",
        spaces(18)
    );
    assert!(lost_continuations(&src).is_empty());
}

#[test]
fn alignment_in_a_literal_that_writes_its_own_newlines_is_not_reported() {
    let src = format!(
        "let m = \"implements:{}{{a}}\\nregistered:{}{{b}}\";\n",
        spaces(8),
        spaces(8)
    );
    assert!(lost_continuations(&src).is_empty());
}

#[test]
fn raw_strings_comments_and_code_are_not_reported() {
    let src = format!(
        "let a = r#\"one{s}two\"#;\n// one{s}two \"\n/* one{s}two \" */\nlet b = 1;{s}let c = 2;\n",
        s = spaces(10)
    );
    assert!(
        lost_continuations(&src).is_empty(),
        "{:?}",
        lost_continuations(&src)
    );
}

#[test]
fn a_quote_in_a_char_literal_does_not_open_a_string() {
    // If `'"'` opened a string, the run of spaces between the two statements
    // would be read as inside one.
    let src = format!(
        "let q = '\"';{}let lifetime: &'static str = \"x\";\n",
        spaces(10)
    );
    assert!(lost_continuations(&src).is_empty());
}

#[test]
fn runs_at_either_end_of_a_literal_are_not_reported() {
    let src = format!("let m = \"{s}padded{s}\";\n", s = spaces(10));
    assert!(lost_continuations(&src).is_empty());
}
