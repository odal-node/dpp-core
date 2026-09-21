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
