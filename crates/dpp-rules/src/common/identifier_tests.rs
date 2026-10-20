//! What [`is_absolute_web_url`](super::identifier::is_absolute_web_url) and
//! [`check_did`](super::identifier::check_did) accept, and the values the
//! plugin tier used to let through before both tiers shared them.

use super::identifier::*;

#[test]
fn an_absolute_web_url_needs_a_host() {
    assert!(is_absolute_web_url("https://example.com/01/09506000134352"));
    assert!(is_absolute_web_url("http://example.com"));
    // The two the prefix-and-remainder test let through.
    assert!(!is_absolute_web_url("https:///p/1"));
    assert!(!is_absolute_web_url("https://?q"));
    assert!(!is_absolute_web_url("https://"));
    assert!(!is_absolute_web_url("https://exa mple.com"));
    assert!(!is_absolute_web_url("ftp://example.com"));
    assert!(!is_absolute_web_url("example.com"));
}

/// 🚨 A link is printable ASCII, and it is refused rather than rewritten.
///
/// EN 18219:2026 clause 4.3 makes a unique identifier an ISO/IEC 646 string, and
/// as a URI it is percent-encoded. These two used to pass: both are a non-empty
/// authority and no whitespace, and nothing looked at what the characters were.
#[test]
fn a_link_with_a_raw_non_ascii_character_is_refused_not_converted() {
    // The two the issue names.
    assert_eq!(
        check_web_url("https://exämple.com/item"),
        Err(WebUrlRejection::NonAsciiHost('ä'))
    );
    assert_eq!(
        check_web_url("https://example.com/ü"),
        Err(WebUrlRejection::NotPrintableAscii('ü'))
    );

    // The same links, written the way the standard asks, are accepted: the path
    // percent-encoded, the host in punycode.
    assert_eq!(check_web_url("https://example.com/%C3%BC"), Ok(()));
    assert_eq!(check_web_url("https://xn--exmple-cua.com/item"), Ok(()));
    assert_eq!(
        check_web_url("https://xn--exmple-cua.com/%C3%BC?q=%C3%A4#%C3%B6"),
        Ok(())
    );

    // Whatever the script: the check is on ASCII, not on one alphabet.
    assert_eq!(
        check_web_url("https://例え.jp/"),
        Err(WebUrlRejection::NonAsciiHost('例'))
    );
    assert_eq!(
        check_web_url("https://example.com/商品/1"),
        Err(WebUrlRejection::NotPrintableAscii('商'))
    );
}

#[test]
fn a_non_ascii_character_is_judged_by_where_it_sits() {
    // The host is what follows the last `@` of the authority. User information
    // before it is percent-encoded, not punycoded, so it gets the other remedy.
    assert_eq!(
        check_web_url("https://üser@example.com/"),
        Err(WebUrlRejection::NotPrintableAscii('ü'))
    );
    assert_eq!(
        check_web_url("https://user@exämple.com/"),
        Err(WebUrlRejection::NonAsciiHost('ä'))
    );
    // An `@` after the authority ends belongs to the path, and moves nothing.
    assert_eq!(
        check_web_url("https://example.com/a@ü"),
        Err(WebUrlRejection::NotPrintableAscii('ü'))
    );
    // The query and the fragment are not the host either.
    assert_eq!(
        check_web_url("https://example.com/?q=é"),
        Err(WebUrlRejection::NotPrintableAscii('é'))
    );
    assert_eq!(
        check_web_url("https://example.com/#é"),
        Err(WebUrlRejection::NotPrintableAscii('é'))
    );
    // The first offender is the one named, wherever the others are.
    assert_eq!(
        check_web_url("https://example.com/ü/é"),
        Err(WebUrlRejection::NotPrintableAscii('ü'))
    );
    assert_eq!(
        check_web_url("https://exämple.com/ü"),
        Err(WebUrlRejection::NonAsciiHost('ä'))
    );
}

#[test]
fn a_control_character_is_outside_printable_ascii_too() {
    assert_eq!(
        check_web_url("https://example.com/\u{1}"),
        Err(WebUrlRejection::NotPrintableAscii('\u{1}'))
    );
    assert_eq!(
        check_web_url("https://example.com/\u{7f}"),
        Err(WebUrlRejection::NotPrintableAscii('\u{7f}'))
    );
    // Vertical tab is a control character, not one of the ASCII whitespace.
    assert_eq!(
        check_web_url("https://example.com/\u{b}"),
        Err(WebUrlRejection::NotPrintableAscii('\u{b}'))
    );
    // An ASCII control character in the host is not a Unicode host.
    assert_eq!(
        check_web_url("https://exa\u{1}mple.com/"),
        Err(WebUrlRejection::NotPrintableAscii('\u{1}'))
    );
}

#[test]
fn whitespace_and_a_missing_host_are_decided_before_any_character() {
    // ASCII whitespace keeps the verdict it always had.
    for bad in [
        "https://exa mple.com",
        "https://example.com/a b",
        "https://example.com/\t",
        "https://example.com/\n",
    ] {
        assert_eq!(
            check_web_url(bad),
            Err(WebUrlRejection::NotAnAbsoluteWebUrl),
            "{bad:?}"
        );
    }
    // No host to resolve, so there is nothing to say about its characters.
    assert_eq!(
        check_web_url("https:///ü"),
        Err(WebUrlRejection::NotAnAbsoluteWebUrl)
    );
    assert_eq!(
        check_web_url("ü://example.com"),
        Err(WebUrlRejection::NotAnAbsoluteWebUrl)
    );
    // Non-ASCII whitespace is a non-ASCII character first, and gets that remedy.
    assert_eq!(
        check_web_url("https://exa\u{a0}mple.com"),
        Err(WebUrlRejection::NonAsciiHost('\u{a0}'))
    );
}

#[test]
fn the_boolean_form_agrees_with_the_check() {
    for url in [
        "https://example.com/01/09506000134352",
        "https://example.com/%C3%BC",
        "https://example.com/ü",
        "https://exämple.com/",
        "https://exa mple.com",
        "https://",
        "example.com",
    ] {
        assert_eq!(
            is_absolute_web_url(url),
            check_web_url(url).is_ok(),
            "{url}"
        );
    }
    assert!(!is_absolute_web_url("https://exämple.com/item"));
    assert!(!is_absolute_web_url("https://example.com/ü"));
}

#[test]
fn a_did_needs_a_named_method_and_a_well_formed_id() {
    assert_eq!(check_did("did:web:example.com"), Ok(()));
    assert_eq!(check_did("did:ethr:0xAbC123"), Ok(()));
    assert_eq!(check_did("did:ebsi:z25a23eWUxQQzmAgnD9srpuU"), Ok(()));
    // A colon-separated id: only the last segment must be non-empty.
    assert_eq!(check_did("did:web:example.com:products:1"), Ok(()));
    assert_eq!(check_did("did:web:ex%2Fample"), Ok(()));

    // The one the non-empty-remainder test let through.
    assert_eq!(check_did("did:web: "), Err(DidRejection::Malformed));
    assert_eq!(check_did("did:web:"), Err(DidRejection::EmptyMethodId));
    assert_eq!(
        check_did("did:key:z6Mk"),
        Err(DidRejection::UnsupportedMethod("key"))
    );
    assert_eq!(check_did("did:web:a:"), Err(DidRejection::Malformed));
    // `%2a` would be a *valid* escape — `a` is a hex digit. The truncated
    // one that is not is `%2z`.
    assert_eq!(
        check_did("did:web:ex%2zample"),
        Err(DidRejection::Malformed)
    );
    assert_eq!(
        check_did("did:web:trailing%2"),
        Err(DidRejection::Malformed)
    );
    assert_eq!(check_did("did:web"), Err(DidRejection::Malformed));
    assert_eq!(check_did("web:example.com"), Err(DidRejection::Malformed));
}

#[test]
fn an_ai_21_serial_is_one_to_twenty_cset_82_characters() {
    assert_eq!(check_gs1_serial("SN-2026/00042"), Ok(()));
    assert_eq!(check_gs1_serial("cdef1032547698badcfe"), Ok(()));
    assert_eq!(check_gs1_serial("!\"%&'()*+,-./:;<=>?_"), Ok(()));

    assert_eq!(check_gs1_serial(""), Err(Gs1ValueRejection::Empty));
    assert_eq!(
        check_gs1_serial("123456789012345678901"),
        Err(Gs1ValueRejection::TooLong { chars: 21 })
    );
    // Counted in characters, not bytes: a two-byte character is one too many
    // for the character set, not two too many for the length.
    assert_eq!(
        check_gs1_serial("é"),
        Err(Gs1ValueRejection::OutsideCset82('é'))
    );
    for outside in [
        ' ', '#', '$', '@', '[', '\\', ']', '^', '`', '{', '|', '}', '~',
    ] {
        assert_eq!(
            check_gs1_serial(&alloc::format!("SN{outside}1")),
            Err(Gs1ValueRejection::OutsideCset82(outside)),
            "{outside:?} is not in CSET 82"
        );
    }
}

/// AI 10 is `X..20` like AI 21, so a lot is held to the same bounds and the
/// same character set — which is what a batch-level carrier prints.
#[test]
fn an_ai_10_lot_is_one_to_twenty_cset_82_characters() {
    assert_eq!(check_gs1_lot("LOT-2026/A"), Ok(()));
    assert_eq!(check_gs1_lot("12345678901234567890"), Ok(()));

    assert_eq!(check_gs1_lot(""), Err(Gs1ValueRejection::Empty));
    assert_eq!(
        check_gs1_lot("123456789012345678901"),
        Err(Gs1ValueRejection::TooLong { chars: 21 })
    );
    assert_eq!(
        check_gs1_lot("LOT A"),
        Err(Gs1ValueRejection::OutsideCset82(' '))
    );
}

/// The table has eighty-two members, as its name says. A count is weak
/// evidence of the *right* members — the GS1 oracle is what checks those —
/// but it catches a character dropped or duplicated in editing.
#[test]
fn cset_82_has_eighty_two_members() {
    let members = (0u8..=127).filter(|b| is_cset_82(char::from(*b))).count();
    assert_eq!(members, 82);
}
