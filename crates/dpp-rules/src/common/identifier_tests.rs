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
