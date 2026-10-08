//! What the parser does with the *value* of each AI and with the URI around it.
//!
//! These are this crate's own checks, written by the people who wrote the parser,
//! and they carry no conformance claim. They pin each rule so a change shows up
//! here; whether the rules are GS1's is what `tests/gs1_syntax_rules_corpus.rs` and
//! GS1's own syntax engine judge.

use dpp_domain::gs1_check_digit;

use super::*;

const BASE: &str = "https://example.com";
const GTIN: &str = "09506000134352";

/// `digits` with its GS1 modulo-10 check digit appended, so a value is valid by
/// construction rather than by a constant that could itself be wrong.
fn with_check(digits: &str) -> String {
    let data: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    format!("{digits}{}", gs1_check_digit(&data))
}

/// A well-formed value for each primary key, keyed by its AI.
fn valid_key_values() -> Vec<(&'static str, String)> {
    vec![
        ("00", with_check("10614141123456789")),
        ("253", with_check("401234567890")),
        ("255", with_check("401234567890")),
        ("401", "4012345ORDER99".into()),
        ("402", with_check("4012345678901234")),
        ("414", with_check("422635080000")),
        ("417", with_check("422635080000")),
        ("8003", format!("0{}", with_check("401234567890"))),
        ("8004", "4012345ABC123".into()),
        ("8006", format!("{GTIN}0101")),
        ("8010", "4012345ABC123".into()),
        ("8013", "1987654Ad4X4bL5ttr2310c2K".into()),
        ("8017", with_check("40123456789012345")),
        ("8018", with_check("40123456789012345")),
    ]
}

fn parse(path: &str) -> Result<DigitalLink, DigitalLinkError> {
    DigitalLink::parse(&format!("{BASE}{path}"))
}

#[test]
fn every_primary_key_parses_with_a_well_formed_value() {
    for (ai, value) in valid_key_values() {
        assert!(
            parse(&format!("/{ai}/{value}")).is_ok(),
            "AI {ai} value {value} should parse: {:?}",
            parse(&format!("/{ai}/{value}")).err()
        );
    }
}

#[test]
fn a_numeric_value_is_held_to_its_length_and_its_digits() {
    for (ai, value) in valid_key_values() {
        // The keys with an optional trailing component take one more character
        // happily; they have a test of their own.
        if !value.bytes().all(|b| b.is_ascii_digit()) || matches!(ai, "253" | "255" | "8003") {
            continue;
        }
        let shorter = &value[..value.len() - 1];
        assert!(
            matches!(
                parse(&format!("/{ai}/{shorter}")),
                Err(DigitalLinkError::ValueTooShort { .. })
            ),
            "AI {ai} one digit short"
        );
        assert!(
            matches!(
                parse(&format!("/{ai}/{value}0")),
                Err(DigitalLinkError::ValueTooLong { .. })
            ),
            "AI {ai} one digit long"
        );
        let lettered = format!("{}A{}", &value[..3], &value[4..]);
        assert!(
            matches!(
                parse(&format!("/{ai}/{lettered}")),
                Err(DigitalLinkError::OutsideCharset { .. })
            ),
            "AI {ai} with a letter in it"
        );
    }
}

#[test]
fn a_check_digit_is_held_wherever_the_dictionary_names_one() {
    for (ai, value) in valid_key_values() {
        // Only the keys whose value *is* the check-digited number: AI 8003 and
        // AI 8006 carry it inside a longer value, and the rest have none.
        if !matches!(ai, "00" | "402" | "414" | "417" | "8017" | "8018") {
            continue;
        }
        let (head, last) = value.split_at(value.len() - 1);
        let wrong = (last.parse::<u8>().unwrap() + 1) % 10;
        assert!(
            matches!(
                parse(&format!("/{ai}/{head}{wrong}")),
                Err(DigitalLinkError::InvalidCheckDigit { .. })
            ),
            "AI {ai} with a wrong check digit"
        );
    }
}

#[test]
fn the_check_digit_covers_its_own_component_not_the_whole_value() {
    // AI 8003 is `N1,zero N13,csum [X..16]`: the digit closing the thirteen digits
    // after the filler is the one checked, and the serial after it is not.
    let key = with_check("401234567890");
    assert!(parse(&format!("/8003/0{key}")).is_ok());
    assert!(parse(&format!("/8003/0{key}ANY-SERIAL")).is_ok());
    let (head, last) = key.split_at(key.len() - 1);
    let wrong = (last.parse::<u8>().unwrap() + 1) % 10;
    assert!(matches!(
        parse(&format!("/8003/0{head}{wrong}ANY-SERIAL")),
        Err(DigitalLinkError::InvalidCheckDigit { .. })
    ));
}

#[test]
fn a_filler_digit_must_be_zero() {
    let key = with_check("401234567890");
    assert!(matches!(
        parse(&format!("/8003/1{key}")),
        Err(DigitalLinkError::NonZeroFiller { .. })
    ));
}

#[test]
fn an_optional_trailing_component_is_held_to_its_own_set_and_length() {
    let key = with_check("401234567890");
    // GCN: thirteen digits, then up to twelve more.
    assert!(parse(&format!("/255/{key}")).is_ok());
    assert!(parse(&format!("/255/{key}123456789012")).is_ok());
    assert!(matches!(
        parse(&format!("/255/{key}1234567890123")),
        Err(DigitalLinkError::ValueTooLong { .. })
    ));
    assert!(matches!(
        parse(&format!("/255/{key}ABC")),
        Err(DigitalLinkError::OutsideCharset { .. })
    ));
    // GDTI: thirteen digits, then up to seventeen characters of CSET 82.
    assert!(parse(&format!("/253/{key}{}", "A".repeat(17))).is_ok());
    assert!(matches!(
        parse(&format!("/253/{key}{}", "A".repeat(18))),
        Err(DigitalLinkError::ValueTooLong { .. })
    ));
    assert!(matches!(
        parse(&format!("/253/{key}A%40B")),
        Err(DigitalLinkError::OutsideCset82 { .. })
    ));
}

#[test]
fn a_cpid_draws_from_the_thirty_nine_character_set() {
    assert!(parse("/8010/4012345ABC-1%23%2F").is_ok());
    for bad in ["4012345abc", "4012345AB_C"] {
        assert!(
            matches!(
                parse(&format!("/8010/{bad}")),
                Err(DigitalLinkError::OutsideCharset {
                    charset: "GS1 CSET 39",
                    ..
                })
            ),
            "{bad}"
        );
    }
}

#[test]
fn a_qualifier_is_held_to_its_own_format() {
    let gln = with_check("422635080000");
    assert!(parse(&format!("/414/{gln}/7040/1ABC")).is_ok());
    assert!(matches!(
        parse(&format!("/414/{gln}/7040/ABCD")),
        Err(DigitalLinkError::OutsideCharset { .. })
    ));
    assert!(matches!(
        parse(&format!("/414/{gln}/7040/1AB")),
        Err(DigitalLinkError::ValueTooShort { .. })
    ));
    assert!(parse("/8010/4012345ABC/8011/123456789012").is_ok());
    assert!(matches!(
        parse("/8010/4012345ABC/8011/ABC"),
        Err(DigitalLinkError::OutsideCharset { .. })
    ));
    assert!(matches!(
        parse("/8010/4012345ABC/8011/1234567890123"),
        Err(DigitalLinkError::ValueTooLong { .. })
    ));
    let gsrn = with_check("40123456789012345");
    assert!(parse(&format!("/8017/{gsrn}/8019/1234567890")).is_ok());
    assert!(matches!(
        parse(&format!("/8017/{gsrn}/8019/12345678901")),
        Err(DigitalLinkError::ValueTooLong { .. })
    ));
}

#[test]
fn a_pay_to_gln_cannot_stand_without_its_payment_reference() {
    let gln = with_check("422635080000");
    assert!(matches!(
        parse(&format!("/415/{gln}")),
        Err(DigitalLinkError::MissingQualifier { .. })
    ));
    assert!(parse(&format!("/415/{gln}/8020/REF123")).is_ok());
    // No other key is dependent: the rule is read from the dictionary, not
    // applied to every key that has a qualifier.
    assert!(parse(&format!("/414/{gln}")).is_ok());
}

#[test]
fn the_grammars_four_scheme_spellings_are_read_and_no_other() {
    for scheme in ["http", "https", "HTTP", "HTTPS"] {
        let link = DigitalLink::parse(&format!("{scheme}://example.com/01/{GTIN}"))
            .unwrap_or_else(|e| panic!("{scheme} must parse: {e}"));
        assert_eq!(link.resolver_base, format!("{scheme}://example.com"));
        assert_eq!(link.build(), format!("{scheme}://example.com/01/{GTIN}"));
    }
    for scheme in ["ftp", "ws", "mailto", "Https", "hTTp"] {
        assert!(
            matches!(
                DigitalLink::parse(&format!("{scheme}://example.com/01/{GTIN}")),
                Err(DigitalLinkError::InvalidScheme(_))
            ),
            "{scheme}"
        );
    }
    assert!(matches!(
        DigitalLink::parse(&format!("example.com/01/{GTIN}")),
        Err(DigitalLinkError::InvalidScheme(_))
    ));
}

#[test]
fn a_user_name_is_not_part_of_the_host() {
    assert!(matches!(
        DigitalLink::parse(&format!("https://user@example.com/01/{GTIN}")),
        Err(DigitalLinkError::InvalidHost(_))
    ));
}

#[test]
fn a_legacy_gtin_is_still_read_and_is_built_back_at_fourteen_digits() {
    // The grammar wants fourteen digits and says only existing infrastructure
    // should keep reading fewer. This reader does, and says so in the register.
    let link = parse("/01/9506000134352").expect("a GTIN-13 is padded");
    assert_eq!(link.build(), format!("{BASE}/01/{GTIN}"));
}

#[test]
fn a_path_symbol_is_read_raw_or_escaped_and_is_built_escaped() {
    let symbols = "!&'()*+,;=:";
    let raw = parse(&format!("/01/{GTIN}/21/A{symbols}B")).expect("raw symbols are read");
    assert_eq!(raw.serial(), Some(format!("A{symbols}B").as_str()));

    let built = raw.build();
    assert_eq!(
        built,
        format!("{BASE}/01/{GTIN}/21/A%21%26%27%28%29%2A%2B%2C%3B%3D%3AB")
    );
    assert_eq!(parse(&built[BASE.len()..]).expect("round trip"), raw);
}

#[test]
fn a_malformed_escape_is_refused_not_read_literally() {
    for bad in ["A%ZZB", "A%2", "A%FFB"] {
        assert!(
            matches!(
                parse(&format!("/01/{GTIN}/21/{bad}")),
                Err(DigitalLinkError::MalformedPercentEscape(_))
            ),
            "{bad}"
        );
    }
}

#[test]
fn a_primary_key_value_is_escaped_when_built() {
    // A CPID may hold `#` and `/`, which would otherwise start a fragment and a
    // new path segment. The builder used to write the primary key raw.
    let link = parse("/8010/4012345AB-C%23%2F").expect("a CPID with YSYMBOLs");
    assert_eq!(link.build(), format!("{BASE}/8010/4012345AB-C%23%2F"));
    assert_eq!(
        parse(&link.build()[BASE.len()..]).expect("it parses back"),
        link
    );
}

/// The pair is the GS1 General Specifications' calculation for alphanumeric keys;
/// GS1's engine agrees on this value, and on every GMN in the oracle corpus.
#[test]
fn a_gmn_is_held_to_its_check_character_pair() {
    assert!(parse("/8013/1987654Ad4X4bL5ttr2310c2K").is_ok());
    assert!(matches!(
        parse("/8013/1987654Ad4X4bL5ttr2310c2X"),
        Err(DigitalLinkError::InvalidCheckPair { expected, actual, .. })
            if expected == "2K" && actual == "2X"
    ));
}

/// Every value is escaped as the builder escapes it, so a symbol reaches the
/// check as itself.
fn escaped(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// GS1's own test vectors for the check character pair, from its syntax engine,
/// that also begin with the four digits AI 8013's `gcppos1` asks for. Between
/// them they weigh every symbol of CSET 82, so a mis-ordered weight fails here.
#[test]
fn the_check_character_pair_agrees_with_gs1s_vectors() {
    for good in [
        "12345678901234567890123NT",
        "12345_ABCDEFGHIJKLMCP",
        "12345_NOPQRSTUVWXYZDN",
        "12345_abcdefghijklmN3",
        "12345_nopqrstuvwxyzP2",
        "12345_!\"%&'()*+,-./LC",
        "12345_0123456789:;<=>?62",
        "7907665Bm8v2AB",
        "97850l6KZm0yCD",
        "225803106GSpEF",
        "149512464PM+GH",
        "62577B8fRG7HJK",
        "515942070CYxLM",
        "390800494sP6NP",
        "386830132uO+QR",
        "53395376X1:nST",
        "957813138Sb6UV",
        "530790no0qOgWX",
        "62185314IvwmYZ",
        "23956qk1&dB!23",
        "794394895ic045",
    ] {
        let path = format!("/8013/{}", escaped(good));
        assert!(parse(&path).is_ok(), "{good} is a GMN with a valid pair");
    }
    assert!(matches!(
        parse("/8013/1987654Ad4X4bL5ttr2310cXK"),
        Err(DigitalLinkError::InvalidCheckPair { .. })
    ));
    assert!(matches!(
        parse("/8013/2"),
        Err(DigitalLinkError::ValueTooShort { min_len: 2, .. })
    ));
}

/// `gcppos1`: the value begins with the four digits of a GS1 Company Prefix.
#[test]
fn a_value_that_cannot_begin_with_a_company_prefix_is_refused() {
    for bad in ["ABCD123", "123", "12A4567"] {
        assert!(
            matches!(
                parse(&format!("/8004/{bad}")),
                Err(DigitalLinkError::InvalidCompanyPrefix { .. })
            ),
            "{bad}"
        );
    }
    assert!(parse("/8004/1234ABC").is_ok());
}
