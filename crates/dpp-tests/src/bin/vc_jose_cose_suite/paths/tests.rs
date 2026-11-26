//! Tests of `paths`.

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
