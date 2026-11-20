//! The suite's disclosure paths, and the `ClaimPath`s `dpp-vc` takes.
//!
//! The suite writes a path as a dotted string, with `[n]` after a member for one array
//! element: `credentialSubject.phoneNumbers[0]`. The library takes `ClaimPath`s and has no
//! string syntax of its own, so the conversion is this harness's. A member name that holds
//! a `.`, `[` or `]` cannot be written in the suite's syntax, and none of its paths has one.

use dpp_crypto::sd_jwt::{ClaimPath, PathSegment};
use serde_json::Value;

/// The `--sd` value, a JSON array of the suite's paths, as `ClaimPath`s.
pub fn parse_sd(text: &str) -> Result<Vec<ClaimPath>, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("--sd is not JSON: {e}"))?;
    let items = value
        .as_array()
        .ok_or("--sd is not a JSON array of strings")?;
    items
        .iter()
        .map(|item| {
            let path = item
                .as_str()
                .ok_or("--sd holds a value that is not a string")?;
            claim_path(path)
        })
        .collect()
}

/// One of the suite's paths: members separated by `.`, each with any number of `[n]`.
pub fn claim_path(path: &str) -> Result<ClaimPath, String> {
    let bad = |why: &str| format!("the path `{path}` {why}");
    let mut segments = Vec::new();
    for part in path.split('.') {
        let name_end = part.find('[').unwrap_or(part.len());
        let (name, mut indices) = part.split_at(name_end);
        if name.is_empty() || name.contains(']') {
            return Err(bad("has an empty or malformed member name"));
        }
        segments.push(PathSegment::Key(name.to_owned()));
        while !indices.is_empty() {
            let (inside, rest) = indices
                .strip_prefix('[')
                .and_then(|after| after.split_once(']'))
                .ok_or_else(|| bad("has an unclosed or stray bracket"))?;
            let position = inside
                .parse::<usize>()
                .ok()
                .filter(|_| inside.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| bad("has a position that is not a non-negative integer"))?;
            segments.push(PathSegment::Index(position));
            indices = rest;
        }
    }
    ClaimPath::new(segments).map_err(|e| bad(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(path: &str) -> String {
        claim_path(path).expect("a path").to_string()
    }

    #[test]
    fn a_dotted_path_is_a_sequence_of_members() {
        assert_eq!(
            json("credentialSubject.firstName"),
            r#"["credentialSubject","firstName"]"#
        );
        assert_eq!(json("holder"), r#"["holder"]"#);
    }

    #[test]
    fn a_bracket_is_one_array_element() {
        assert_eq!(
            json("credentialSubject.phoneNumbers[0]"),
            r#"["credentialSubject","phoneNumbers",0]"#
        );
        assert_eq!(
            json("verifiableCredential[0]"),
            r#"["verifiableCredential",0]"#
        );
        assert_eq!(json("a[1][20].b"), r#"["a",1,20,"b"]"#);
    }

    #[test]
    fn the_suites_six_paths_convert() {
        let converted = parse_sd(
            r#"["credentialSubject.address.street", "credentialSubject.address.city",
                "credentialSubject.phoneNumbers[0]", "holder", "verifiableCredential[0]",
                "credentialSubject.lastName"]"#,
        )
        .expect("converts");

        let text: Vec<String> = converted.iter().map(ToString::to_string).collect();
        assert_eq!(
            text,
            [
                r#"["credentialSubject","address","street"]"#,
                r#"["credentialSubject","address","city"]"#,
                r#"["credentialSubject","phoneNumbers",0]"#,
                r#"["holder"]"#,
                r#"["verifiableCredential",0]"#,
                r#"["credentialSubject","lastName"]"#,
            ]
        );
    }

    #[test]
    fn a_path_that_is_not_in_the_suites_syntax_is_refused() {
        for path in [
            "", ".", "a.", ".a", "a..b", "[0]", "a[", "a[]", "a[x]", "a[-1]", "a[+1]", "a[1", "a]",
            "a[1]b", "a[ 1]", "a[1.5]",
        ] {
            assert!(claim_path(path).is_err(), "{path:?}");
        }
    }

    #[test]
    fn the_sd_value_must_be_a_json_array_of_strings() {
        assert_eq!(parse_sd("[]").map(|p| p.len()), Ok(0));
        for text in [
            "",
            "{}",
            r#""a.b""#,
            "[1]",
            r#"[["a"]]"#,
            r#"["a", null]"#,
            "not json",
        ] {
            assert!(parse_sd(text).is_err(), "{text:?}");
        }
    }
}
