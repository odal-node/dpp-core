//! Whether a string is an absolute URL, for the `kid` rule of VC-JOSE-COSE §4.2.

/// Whether `text` starts with a URI scheme (RFC 3986 §3.1) followed by a colon, and
/// holds no whitespace or control character.
///
/// This is the syntactic test for the *absolute* form, with no network and no URL
/// parser: a DID URL, an `https` URL and a URN all pass, and a bare thumbprint does
/// not. It does not say the URL resolves to anything.
pub(crate) fn is_absolute_url(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        && !rest.is_empty()
        && !text.chars().any(|c| c.is_whitespace() || c.is_control())
}
