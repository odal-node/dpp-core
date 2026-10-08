//! One whole URI for each rule of GS1 Digital Link URI Syntax 1.7.0 section 4,
//! with the verdict the grammar gives it, for GS1's own syntax engine to judge.
//!
//! # Why this exists
//!
//! `gs1_oracle_corpus.rs` hands the engine every link this crate builds, and a few
//! it must refuse, and settled the characters of CSET 82. This is the rest of the
//! path grammar: the value format of each primary key and of each qualifier, the
//! order and the alternatives of the qualifiers, the composite paths, the scheme
//! and the host, the percent escapes, and the extension parameters of the query.
//! Section 2 of the standard says conformance is defined by that grammar and names
//! the GS1 Barcode Syntax Resource as a tool to confirm it with. The Resource is a
//! dictionary, a set of linters and this engine, which runs them.
//!
//! # What each case carries
//!
//! - `grammar`: whether the URI conforms, which is this file's reading of the
//!   grammar and of the GS1 General Specifications rules it points to (a check
//!   digit, for one). It is the one thing here that is not independent.
//! - `ours`: what [`DigitalLink::parse`] says, recorded rather than declared.
//! - `ours_differs` / `engine_differs`: a reason, present exactly where that tool's
//!   verdict is not the grammar's. Every deviation of this crate is one of these
//!   strings, and so is every place the engine is more lenient or stricter than the
//!   grammar, so neither list can be kept anywhere but here.
//!
//! The test below asserts `ours` against `grammar` and the annotations. The oracle
//! asserts the engine's verdict against them. A difference nobody wrote down fails
//! one side; a written-down difference that has stopped being true fails too, so
//! the lists cannot go stale.
//!
//! # Scope
//!
//! The path. The query string is read by nothing in this crate, so it appears only
//! as extension parameters, which are the one kind a resolver sends, and as four
//! data-attribute errors that record the deviation.
//!
//! # Running
//!
//! ```text
//! EMIT_GS1_CORPUS=1 cargo test -p dpp-digital-link --test gs1_syntax_rules_corpus
//! ```
//!
//! Writes `target/gs1-oracle/rules.jsonl`. Without the variable the test still
//! runs and still checks our own side.

use dpp_digital_link::DigitalLink;
use dpp_domain::gs1_check_digit;

const BASE: &str = "https://example.com";
const GTIN: &str = "09506000134352";

struct Case {
    uri: String,
    rule: &'static str,
    grammar: bool,
    ours_differs: Option<&'static str>,
    engine_differs: Option<&'static str>,
    /// Also judged with the engine's mode for legacy zero-suppressed GTINs.
    legacy: bool,
}

fn case(uri: impl Into<String>, rule: &'static str, grammar: bool) -> Case {
    Case {
        uri: uri.into(),
        rule,
        grammar,
        ours_differs: None,
        engine_differs: None,
        legacy: false,
    }
}

impl Case {
    fn ours(mut self, reason: &'static str) -> Self {
        self.ours_differs = Some(reason);
        self
    }
    fn engine(mut self, reason: &'static str) -> Self {
        self.engine_differs = Some(reason);
        self
    }
    fn legacy(mut self) -> Self {
        self.legacy = true;
        self
    }
}

/// `digits` with its GS1 modulo-10 check digit appended, so a value is valid by
/// construction rather than by a constant that could itself be wrong.
fn with_check(digits: &str) -> String {
    let data: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    format!("{digits}{}", gs1_check_digit(&data))
}

/// The same digits with the check digit moved off its right value.
fn wrong_check(valid: &str) -> String {
    let (head, last) = valid.split_at(valid.len() - 1);
    format!("{head}{}", (last.parse::<u8>().expect("a digit") + 1) % 10)
}

fn at(path: &str) -> String {
    format!("{BASE}{path}")
}

const NOT_DL: &str = "the engine parses only an http or https input as a Digital Link, so any other string is plain data to it";
const LEGACY_GTIN: &str = "a GTIN of fewer than 14 digits is the legacy form: the grammar wants 14, and this reader still pads it, as the standard allows existing infrastructure to";
const TRAILING_SLASH: &str = "a trailing slash is not in the grammar, and this reader tolerates it, as the resolver standard asks resolvers to";
const EMPTY_HOST: &str = "an empty host is refused: the grammar's reg-name may be empty, but that names no resolver and cannot be built back into a link";
const QUERY_UNREAD: &str = "the query string is not read, so a malformed one is accepted";
const ENGINE_ESCAPES: &str = "the engine reads a malformed percent escape as literal characters";
const ENGINE_RAW: &str = "the engine reads the sub-delimiters raw as well as escaped";
const RAW_SYMBOL: &str = "a symbol the grammar spells as an escape is read raw too, since RFC 3986 allows it in a path; it is always built escaped";
const ENGINE_STEM: &str = "the engine holds the custom path stem only to the characters a URI may hold, not to a segment's";
const ENGINE_DOMAIN: &str = "the engine reads a host as a domain name, and refuses the sub-delimiters and escapes reg-name allows";
const NO_MATCHING_REQ: &str = "the engine's requisite-AI check wants AI 01 for a qualifier of AI 8006, which the grammar's path for an ITIP allows";

#[allow(clippy::too_many_lines)]
fn corpus() -> Vec<Case> {
    let mut out: Vec<Case> = Vec::new();

    // ── scheme and host (4.11) ──────────────────────────────────────────────
    for scheme in ["https", "http", "HTTPS", "HTTP"] {
        out.push(case(
            format!("{scheme}://example.com/01/{GTIN}"),
            "4.11 scheme",
            true,
        ));
    }
    for host in [
        "example.com:8443",
        "id.gs1.org",
        "127.0.0.1:80",
        "[::1]",
        "[2001:db8::1]:8443",
        "example.com/a/b/c",
    ] {
        out.push(case(
            format!("https://{host}/01/{GTIN}"),
            "4.11 hostname and custom path",
            true,
        ));
    }
    out.push(case(
        format!("{BASE}//01/{GTIN}"),
        "4.11 optionalPathSegment may be empty",
        true,
    ));
    for stem in ["a%20b", "r:e@s!$&'()*+,;="] {
        out.push(case(
            format!("{BASE}/{stem}/01/{GTIN}"),
            "4.11 a stem segment is pchar",
            true,
        ));
    }
    for bad in [" ", "\u{e9}", "\"", "<", "{", "\\"] {
        out.push(case(
            format!("{BASE}/re{bad}s/01/{GTIN}"),
            "4.11 a stem segment is pchar",
            false,
        ));
    }
    for bad in ["[", "]", "%zz"] {
        out.push(
            case(
                format!("{BASE}/re{bad}s/01/{GTIN}"),
                "4.11 a stem segment is pchar",
                false,
            )
            .engine(ENGINE_STEM),
        );
    }
    for host in ["exa(mple.com", "exa%41mple.com"] {
        out.push(
            case(
                format!("https://{host}/01/{GTIN}"),
                "4.11 reg-name admits sub-delims and escapes",
                true,
            )
            .engine(ENGINE_DOMAIN),
        );
    }
    out.push(case(format!("ftp://example.com/01/{GTIN}"), "4.11 scheme", false).engine(NOT_DL));
    out.push(case(format!("example.com/01/{GTIN}"), "4.11 scheme", false).engine(NOT_DL));
    out.push(
        case(
            format!("Https://example.com/01/{GTIN}"),
            "4.11 scheme",
            false,
        )
        .engine(NOT_DL),
    );
    out.push(case(
        format!("https://user@example.com/01/{GTIN}"),
        "4.11 hostname has no user name",
        false,
    ));
    out.push(case(
        format!("https://exa mple.com/01/{GTIN}"),
        "4.11 reg-name",
        false,
    ));
    out.push(
        case(
            format!("https://example.com:80a/01/{GTIN}"),
            "4.11 port is digits",
            false,
        )
        .engine("the engine does not check the port"),
    );
    out.push(
        case(
            format!("https:///01/{GTIN}"),
            "4.11 reg-name may be empty",
            true,
        )
        .ours(EMPTY_HOST)
        .engine("the engine requires a domain"),
    );

    // ── GTIN, 4.1 and 4.5: exactly 14 digits ────────────────────────────────
    out.push(case(at(&format!("/01/{GTIN}")), "4.5 gtin-value", true));
    out.push(case(
        at(&format!("/01/{}", wrong_check(GTIN))),
        "4.5 gtin-value check digit",
        false,
    ));
    out.push(case(
        at(&format!("/01/{GTIN}0")),
        "4.5 gtin-value 15 digits",
        false,
    ));
    out.push(case(
        at("/01/0950600013435A"),
        "4.5 gtin-value digits only",
        false,
    ));
    out.push(case(at("/01/"), "4.5 gtin-value present", false));
    // The same GTIN as the legacy shorter forms, by dropping leading zeros of a
    // 14-digit number whose check digit is then unchanged.
    let padded = with_check("0000001234567");
    // GS1's engine has a mode that reads all three, which the oracle holds it to.
    for digits in [1usize, 2, 6] {
        let short = padded[digits..].to_owned();
        out.push(
            case(
                at(&format!("/01/{short}")),
                "4.1 GTIN-8, -12 and -13 are written with 14 digits",
                false,
            )
            .ours(LEGACY_GTIN)
            .legacy(),
        );
    }

    // ── qualifiers of AI 01 (4.4, 4.6, 4.9) ─────────────────────────────────
    let q = |path: &str| at(&format!("/01/{GTIN}{path}"));
    for (path, ok, rule) in [
        ("/22/A", true, "4.9 gtin-path [cpv]"),
        ("/10/B", true, "4.9 gtin-path [lot]"),
        ("/21/C", true, "4.9 gtin-path [ser]"),
        ("/22/A/10/B", true, "4.9 gtin-path cpv lot"),
        ("/22/A/21/C", true, "4.9 gtin-path cpv ser"),
        ("/10/B/21/C", true, "4.9 gtin-path lot ser"),
        ("/22/A/10/B/21/C", true, "4.9 gtin-path all three"),
        ("/235/TPX", true, "4.9 upui-path"),
        ("/10/B/22/A", false, "4.9 gtin-path order: lot before cpv"),
        ("/21/C/10/B", false, "4.9 gtin-path order: ser before lot"),
        ("/21/C/22/A", false, "4.9 gtin-path order: ser before cpv"),
        ("/10/A/10/B", false, "4.9 gtin-path: lot repeated"),
        ("/22/A/22/B", false, "4.9 gtin-path: cpv repeated"),
        ("/21/C/235/T", false, "4.9 upui-path: ser with tpx"),
        ("/22/A/235/T", false, "4.9 upui-path: cpv with tpx"),
        ("/10/B/235/T", false, "4.9 upui-path: lot with tpx"),
        ("/235/T/21/C", false, "4.9 upui-path: tpx with ser"),
        ("/235/T/235/U", false, "4.9 upui-path: tpx repeated"),
        ("/10", false, "4.9 a qualifier needs its value"),
        ("/10/", false, "4.6 lot-value is 1 to 20 characters"),
    ] {
        out.push(case(q(path), rule, ok));
    }
    for (ai, max) in [("22", 20), ("10", 20), ("21", 20), ("235", 28)] {
        out.push(case(
            q(&format!("/{ai}/{}", "A".repeat(max))),
            "4.6 qualifier at its longest",
            true,
        ));
        out.push(case(
            q(&format!("/{ai}/{}", "A".repeat(max + 1))),
            "4.6 qualifier one too long",
            false,
        ));
    }

    // ── the other primary keys, and each of their own qualifiers ───────────
    let sscc = with_check("10614141123456789");
    let gln = with_check("422635080000");
    let gdti = with_check("401234567890");
    let gsin = with_check("4012345678901234");
    let gsrn = with_check("40123456789012345");
    let grai = format!("0{}", with_check("401234567890"));
    let itip = format!("{GTIN}0101");

    // The numeric, fixed-length keys: a valid value, one digit short, one long, a
    // letter inside, and a wrong check digit.
    for (ai, value, key) in [
        ("00", sscc.clone(), "sscc-value 18 digits"),
        ("402", gsin.clone(), "gsin-value 17 digits"),
        ("414", gln.clone(), "gln-value 13 digits"),
        ("417", gln.clone(), "partyGln-value 13 digits"),
        ("8017", gsrn.clone(), "gsrnp-value 18 digits"),
        ("8018", gsrn.clone(), "gsrn-value 18 digits"),
    ] {
        out.push(case(at(&format!("/{ai}/{value}")), key, true));
        out.push(case(
            at(&format!("/{ai}/{}", &value[..value.len() - 1])),
            key,
            false,
        ));
        out.push(case(at(&format!("/{ai}/{value}0")), key, false));
        out.push(case(
            at(&format!("/{ai}/{}A{}", &value[..4], &value[5..])),
            key,
            false,
        ));
        out.push(case(
            at(&format!("/{ai}/{}", wrong_check(&value))),
            key,
            false,
        ));
    }
    // ITIP: the check digit is the fourteenth digit.
    out.push(case(
        at(&format!("/8006/{itip}")),
        "itip-value 18 digits",
        true,
    ));
    out.push(case(
        at(&format!("/8006/{}", &itip[..itip.len() - 1])),
        "itip-value 18 digits",
        false,
    ));
    out.push(case(
        at(&format!("/8006/{itip}0")),
        "itip-value 18 digits",
        false,
    ));
    out.push(case(
        at(&format!("/8006/{}A{}", &itip[..5], &itip[6..])),
        "itip-value 18 digits",
        false,
    ));
    out.push(case(
        at(&format!(
            "/8006/{}{}",
            wrong_check(&itip[..14]),
            &itip[14..]
        )),
        "itip-value check digit",
        false,
    ));
    for (path, ok) in [("/10/B", true), ("/21/C", true)] {
        out.push(case(
            at(&format!("/8006/{itip}{path}")),
            "4.9 itip-path",
            ok,
        ));
    }
    out.push(
        case(at(&format!("/8006/{itip}/22/A")), "4.9 itip-path cpv", true).engine(NO_MATCHING_REQ),
    );

    // GDTI, GCN, GRAI: a number, then an optional tail of its own kind.
    out.push(case(
        at(&format!("/253/{gdti}")),
        "gdti-value 13 digits",
        true,
    ));
    out.push(case(
        at(&format!("/253/{gdti}ABC123")),
        "gdti-value serial",
        true,
    ));
    out.push(case(
        at(&format!("/253/{gdti}{}", "A".repeat(17))),
        "gdti-value serial at its longest",
        true,
    ));
    out.push(case(
        at(&format!("/253/{gdti}{}", "A".repeat(18))),
        "gdti-value serial one too long",
        false,
    ));
    out.push(case(
        at(&format!("/253/{}", &gdti[..12])),
        "gdti-value 13 digits",
        false,
    ));
    out.push(case(
        at(&format!("/253/{}", wrong_check(&gdti))),
        "gdti-value check digit",
        false,
    ));
    out.push(case(
        at(&format!("/255/{gdti}")),
        "gcn-value 13 digits",
        true,
    ));
    out.push(case(
        at(&format!("/255/{gdti}123456789012")),
        "gcn-value serial at its longest",
        true,
    ));
    out.push(case(
        at(&format!("/255/{gdti}1234567890123")),
        "gcn-value serial one too long",
        false,
    ));
    out.push(case(
        at(&format!("/255/{gdti}ABC")),
        "gcn-value serial is digits",
        false,
    ));
    out.push(case(
        at(&format!("/255/{}", wrong_check(&gdti))),
        "gcn-value check digit",
        false,
    ));
    out.push(case(at(&format!("/8003/{grai}")), "grai-value", true));
    out.push(case(
        at(&format!("/8003/{grai}ABC")),
        "grai-value serial",
        true,
    ));
    out.push(case(
        at(&format!("/8003/{grai}{}", "A".repeat(16))),
        "grai-value serial at its longest",
        true,
    ));
    out.push(case(
        at(&format!("/8003/{grai}{}", "A".repeat(17))),
        "grai-value serial one too long",
        false,
    ));
    out.push(case(
        at(&format!("/8003/{}", &grai[1..])),
        "grai-comp begins with a 0",
        false,
    ));
    out.push(case(
        at(&format!("/8003/{}", &grai[..13])),
        "grai-value 13 digits",
        false,
    ));
    out.push(case(
        at(&format!("/8003/0{}", wrong_check(&gdti))),
        "grai-value check digit",
        false,
    ));

    // The alphanumeric keys.
    for (ai, ok, bad_len, rule) in [
        ("401", "4012345ORDER99", 31usize, "ginc-value 1 to 30"),
        ("8004", "4012345ABC123", 31, "giai-value 1 to 30"),
    ] {
        out.push(case(at(&format!("/{ai}/{ok}")), rule, true));
        out.push(case(
            at(&format!("/{ai}/4012345{}", "A".repeat(23))),
            rule,
            true,
        ));
        out.push(case(
            at(&format!("/{ai}/4012345{}", "A".repeat(bad_len - 7))),
            rule,
            false,
        ));
        out.push(case(at(&format!("/{ai}/4012345A%40B")), rule, false));
    }
    out.push(case(
        at("/8013/1987654Ad4X4bL5ttr2310c2K"),
        "gmn-value",
        true,
    ));
    out.push(case(
        at(&format!("/8013/4012345{}", "A".repeat(19))),
        "gmn-value 25 characters is the most",
        false,
    ));
    out.push(case(
        at("/8013/1987654Ad4X4bL5ttr2310c2X"),
        "gmn-value check character pair",
        false,
    ));
    // GS1's own vector, every symbol escaped: the pair weighs each one.
    out.push(case(
        at("/8013/12345_%21%22%25%26%27%28%29%2A%2B%2C-.%2FLC"),
        "gmn-value check character pair over the symbols of CSET 82",
        true,
    ));
    out.push(case(
        at("/8004/ABCD123"),
        "giai-value begins with a GS1 Company Prefix",
        false,
    ));

    // CPID: GS1 CSET 39.
    out.push(case(at("/8010/4012345ABC123"), "cpid-value", true));
    out.push(case(
        at("/8010/4012345AB-C%23%2F"),
        "cpid-value YSYMBOL",
        true,
    ));
    out.push(case(
        at(&format!("/8010/4012345{}", "A".repeat(23))),
        "cpid-value 30 characters",
        true,
    ));
    out.push(case(
        at(&format!("/8010/4012345{}", "A".repeat(24))),
        "cpid-value 31 characters",
        false,
    ));
    out.push(case(at("/8010/4012345abc"), "cpid-value is CSET 39", false));
    out.push(case(
        at("/8010/4012345AB_C"),
        "cpid-value is CSET 39",
        false,
    ));

    // Qualifiers of the other keys, and the composite paths.
    out.push(case(
        at("/8010/4012345ABC/8011/123456789012"),
        "4.9 cpid-path cpsn",
        true,
    ));
    out.push(case(
        at("/8010/4012345ABC/8011/1234567890123"),
        "4.6 cpsn-value 1 to 12 digits",
        false,
    ));
    out.push(case(
        at("/8010/4012345ABC/8011/ABC"),
        "4.6 cpsn-value is digits",
        false,
    ));
    out.push(case(
        at(&format!("/414/{gln}/254/{}", "A".repeat(20))),
        "4.9 gln-path glnx",
        true,
    ));
    out.push(case(
        at(&format!("/414/{gln}/254/{}", "A".repeat(21))),
        "4.6 glnx-value 1 to 20",
        false,
    ));
    for key in [
        format!("414/{gln}"),
        format!("417/{gln}"),
        "8004/4012345ABC123".to_owned(),
    ] {
        out.push(case(
            at(&format!("/{key}/7040/1ABC")),
            "4.9 fid, eoid and mid paths",
            true,
        ));
        out.push(case(
            at(&format!("/{key}/7040/ABCD")),
            "4.6 uic-ext-value starts with a digit",
            false,
        ));
        out.push(case(
            at(&format!("/{key}/7040/1AB")),
            "4.6 uic-ext-value is 1 digit and 3 characters",
            false,
        ));
        out.push(case(
            at(&format!("/{key}/7040/1ABCD")),
            "4.6 uic-ext-value is 1 digit and 3 characters",
            false,
        ));
    }
    out.push(case(
        at(&format!("/414/{gln}/254/E/7040/1ABC")),
        "4.9 fid-path: glnx with uic",
        false,
    ));
    out.push(case(
        at(&format!("/417/{gln}/254/E")),
        "4.9 partyGln-path takes only a uic",
        false,
    ));
    out.push(case(
        at(&format!("/8017/{gsrn}/8019/1234567890")),
        "4.6 srin-value 1 to 10 digits",
        true,
    ));
    out.push(case(
        at(&format!("/8018/{gsrn}/8019/123")),
        "4.9 gsrn-path srin",
        true,
    ));
    out.push(case(
        at(&format!("/8017/{gsrn}/8019/12345678901")),
        "4.6 srin-value 1 to 10 digits",
        false,
    ));
    out.push(case(
        at(&format!("/8017/{gsrn}/8019/ABC")),
        "4.6 srin-value is digits",
        false,
    ));

    // AI 415 cannot stand without its payment reference.
    out.push(case(
        at(&format!("/415/{gln}/8020/REF123")),
        "4.9 payTo-path needs refNo",
        true,
    ));
    out.push(case(
        at(&format!("/415/{gln}")),
        "4.9 payTo-path needs refNo",
        false,
    ));
    out.push(case(
        at(&format!("/415/{gln}/8020/{}", "A".repeat(25))),
        "4.6 refno-value 1 to 25",
        true,
    ));
    out.push(case(
        at(&format!("/415/{gln}/8020/{}", "A".repeat(26))),
        "4.6 refno-value 1 to 25",
        false,
    ));
    out.push(case(
        at("/8020/REF123"),
        "4.9 a qualifier cannot open a path",
        false,
    ));

    // Structure the path may not have.
    out.push(case(
        at(&format!("/00/{sscc}/22/A")),
        "4.9 sscc-path has no qualifier",
        false,
    ));
    out.push(case(
        at("/8013/1987654Ad4X4bL5ttr2310c2K/10/1"),
        "4.9 gmn-path has no qualifier",
        false,
    ));
    out.push(case(
        at(&format!("/01/{GTIN}/99/DATA")),
        "4.10 a data attribute is not in the path",
        false,
    ));
    out.push(
        case(
            at(&format!("/01/{GTIN}/00/{sscc}")),
            "4.9 one primary key per path",
            false,
        )
        .engine("the engine reads a second primary key as more data"),
    );
    out.push(
        case(
            at(&format!("/00/{sscc}/01/{GTIN}")),
            "4.9 one primary key per path",
            false,
        )
        .engine("the engine reads a second primary key as more data"),
    );
    out.push(case(
        at(&format!("/01/{GTIN}/04/NOSUCHAI")),
        "an unassigned AI",
        false,
    ));
    out.push(
        case(
            at(&format!("/01/{GTIN}/21/C/")),
            "4.12 no trailing slash",
            false,
        )
        .ours(TRAILING_SLASH),
    );
    for path in [format!("/01//{GTIN}"), format!("/01/{GTIN}//21/A")] {
        out.push(case(at(&path), "4.12 no empty segment in the path", false));
    }

    // ── characters of a value (4.2) ─────────────────────────────────────────
    // Every symbol of CSET 82 that is not `-`, `.` or `_`, in its escaped form.
    for (symbol, escaped) in [
        ('!', "%21"),
        ('"', "%22"),
        ('%', "%25"),
        ('&', "%26"),
        ('\'', "%27"),
        ('(', "%28"),
        (')', "%29"),
        ('*', "%2A"),
        ('+', "%2B"),
        (',', "%2C"),
        ('/', "%2F"),
        (':', "%3A"),
        (';', "%3B"),
        ('<', "%3C"),
        ('=', "%3D"),
        ('>', "%3E"),
        ('?', "%3F"),
    ] {
        out.push(case(
            at(&format!("/01/{GTIN}/21/A{escaped}B")),
            "4.2 an escaped symbol of CSET 82",
            true,
        ));
        // The spellings the grammar does not name but RFC 3986 allows raw.
        if "!&'()*+,;=:".contains(symbol) {
            out.push(
                case(
                    at(&format!("/01/{GTIN}/21/A{symbol}B")),
                    "4.2 a symbol the grammar spells as an escape, written raw",
                    false,
                )
                .ours(RAW_SYMBOL)
                .engine(ENGINE_RAW),
            );
        }
    }
    for symbol in ["-", ".", "_"] {
        out.push(case(
            at(&format!("/01/{GTIN}/21/A{symbol}B")),
            "4.2 a symbol of CSET 82 written as itself",
            true,
        ));
    }
    out.push(
        case(
            at(&format!("/01/{GTIN}/21/A\"B")),
            "4.2 XSYMBOL names the double quote itself",
            true,
        )
        .engine("the engine refuses a raw double quote, which RFC 3986 forbids in a URI"),
    );
    // RFC 3986 allows none of these raw anywhere in a URI. The grammar writes `<`
    // and `>` only as escapes, and the rest are outside CSET 82.
    for raw in ['<', '>', '{', '|', '\\', '^', '`'] {
        out.push(case(
            at(&format!("/01/{GTIN}/21/A{raw}B")),
            "4.2 a character no URI holds raw",
            false,
        ));
    }
    out.push(case(
        at(&format!("/01/{GTIN}/21/a%2fb")),
        "4.2 an escape in lower-case hexadecimal",
        true,
    ));
    for bad in ["%24", "%7E", "%40", "%23", "%20", "%C3%A9", "%FF"] {
        out.push(case(
            at(&format!("/01/{GTIN}/21/A{bad}B")),
            "4.2 a character outside CSET 82",
            false,
        ));
    }
    for bad in ["%ZZ", "%2", "%"] {
        out.push(
            case(
                at(&format!("/01/{GTIN}/21/A{bad}")),
                "4.2 a malformed percent escape",
                false,
            )
            .engine(ENGINE_ESCAPES),
        );
    }

    // ── the query string (4.10, 4.11) ───────────────────────────────────────
    for query in [
        "?linkType=gs1:pip",
        "?context=eu",
        "?23P=12098",
        "?linkType=gs1:pip&context=eu",
    ] {
        out.push(case(
            at(&format!("/01/{GTIN}{query}")),
            "4.10 extensionParameter",
            true,
        ));
    }
    for (query, rule) in [
        ("?236=12098", "4.10 an extension key is not all digits"),
        ("?04=X", "4.10 an extension key is not all digits"),
        ("?17=26123", "4.10 expiryDateParameter is 6 digits"),
        ("?3103=500", "4.10 netWeightVMTIParameter is 6 digits"),
    ] {
        out.push(case(at(&format!("/01/{GTIN}{query}")), rule, false).ours(QUERY_UNREAD));
    }
    for (query, rule) in [
        ("?", "4.11 queryStringComp needs a parameter"),
        ("?linkType", "4.10 extensionParameter needs ="),
        ("?linkType=gs1:pip#frag", "4.11 the grammar has no fragment"),
    ] {
        out.push(
            case(at(&format!("/01/{GTIN}{query}")), rule, false)
                .ours(QUERY_UNREAD)
                .engine("the engine does not check the shape of the query beyond its numeric keys"),
        );
    }
    out.push(
        case(
            at(&format!("/01/{GTIN}#frag")),
            "4.11 the grammar has no fragment",
            false,
        )
        .engine("the engine ignores a fragment"),
    );

    out
}

/// Our own side must match the grammar wherever nobody wrote down a difference,
/// and must differ where somebody did.
#[test]
fn our_parser_gives_the_grammars_verdict_except_where_a_difference_is_recorded() {
    let cases = corpus();
    assert!(
        cases.len() > 200,
        "a corpus this small asks little: {}",
        cases.len()
    );

    let mut stale = Vec::new();
    let mut wrong = Vec::new();
    for c in &cases {
        let ours = DigitalLink::parse(&c.uri).is_ok();
        match (c.ours_differs.is_some(), ours == c.grammar) {
            (false, false) => wrong.push(format!(
                "{} [{}]: the grammar says {}, this parser says {}",
                c.uri, c.rule, c.grammar, ours
            )),
            (true, true) => stale.push(format!(
                "{} [{}]: a difference is recorded but the verdicts agree",
                c.uri, c.rule
            )),
            _ => {}
        }
    }
    assert!(
        wrong.is_empty(),
        "unrecorded differences from the grammar:\n{}",
        wrong.join("\n")
    );
    assert!(
        stale.is_empty(),
        "recorded differences that no longer exist:\n{}",
        stale.join("\n")
    );
}

/// Every built link round-trips, so the oracle never reports our bug as GS1's.
#[test]
fn a_link_we_accept_builds_back_to_a_link_we_accept() {
    for c in corpus() {
        let Ok(link) = DigitalLink::parse(&c.uri) else {
            continue;
        };
        let rebuilt = link.build();
        let again = DigitalLink::parse(&rebuilt)
            .unwrap_or_else(|e| panic!("{} built {rebuilt}, which does not parse: {e}", c.uri));
        assert_eq!(
            link, again,
            "{} did not round-trip through {rebuilt}",
            c.uri
        );
    }
}

/// Write the corpus for the external engine, when asked.
#[test]
fn emit_rules_corpus_for_the_gs1_syntax_engine() {
    if std::env::var_os("EMIT_GS1_CORPUS").is_none() {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/gs1-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");

    let mut body = String::new();
    for c in corpus() {
        let line = serde_json::json!({
            "uri": c.uri,
            "accepted": DigitalLink::parse(&c.uri).is_ok(),
            "rule": c.rule,
            "grammar": c.grammar,
            "oursDiffers": c.ours_differs,
            "engineDiffers": c.engine_differs,
            "legacy": c.legacy,
        });
        body.push_str(&line.to_string());
        body.push('\n');
    }
    std::fs::write(dir.join("rules.jsonl"), body).expect("write the corpus");
}
