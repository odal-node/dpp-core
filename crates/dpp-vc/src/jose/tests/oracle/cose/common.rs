//! Reading a `COSE_Sign1` message apart without checking anything: the algorithm of its
//! protected header, and its signature.
//!
//! It reads only what the two writers of the corpus produce, a tagged message whose
//! unprotected header is empty, and panics on anything else, since a corpus that is not that
//! is a mistake to be found.

/// What a message says about itself.
pub(super) struct Parts {
    /// The protected `alg`.
    pub(super) alg: i64,
    /// Where the signature starts: it is the last item, and runs to the end.
    signature_at: usize,
}

/// An item's major type and argument, and the position after the head.
fn head(bytes: &[u8], at: usize) -> (u8, u64, usize) {
    let first = bytes[at];
    let (major, info) = (first >> 5, first & 0x1f);
    match info {
        0..=23 => (major, u64::from(info), at + 1),
        24 => (major, u64::from(bytes[at + 1]), at + 2),
        25 => (
            major,
            u64::from(u16::from_be_bytes([bytes[at + 1], bytes[at + 2]])),
            at + 3,
        ),
        26 => (
            major,
            u64::from(u32::from_be_bytes(
                bytes[at + 1..at + 5].try_into().expect("4 bytes"),
            )),
            at + 5,
        ),
        _ => panic!("an item head this reader does not know: {first:#x}"),
    }
}

/// The position after the item at `at`, of any kind a header of the corpus holds.
fn skip(bytes: &[u8], at: usize) -> usize {
    let (major, argument, mut next) = head(bytes, at);
    let count = usize::try_from(argument).expect("fits");
    match major {
        0 | 1 => next,
        2 | 3 => next + count,
        4 => {
            for _ in 0..count {
                next = skip(bytes, next);
            }
            next
        }
        5 => {
            for _ in 0..2 * count {
                next = skip(bytes, next);
            }
            next
        }
        _ => panic!("an item of major type {major} that this reader does not know"),
    }
}

pub(super) fn parts(message: &[u8]) -> Parts {
    let mut at = 0;
    if message[at] == 0xd2 {
        at += 1;
    }
    let (major, length, next) = head(message, at);
    assert_eq!((major, length), (4, 4), "a COSE_Sign1 array");
    let (major, length, start) = head(message, next);
    assert_eq!(major, 2, "the protected header is a byte string");
    let protected = &message[start..start + usize::try_from(length).expect("fits")];
    at = start + protected.len();
    let (major, pairs, next) = head(message, at);
    assert_eq!((major, pairs), (5, 0), "an empty unprotected header");
    let (major, length, start) = head(message, next);
    assert_eq!(major, 2, "the payload is a byte string");
    at = start + usize::try_from(length).expect("fits");
    let (major, length, start) = head(message, at);
    assert_eq!(major, 2, "the signature is a byte string");
    assert_eq!(
        start + usize::try_from(length).expect("fits"),
        message.len()
    );

    let (major, pairs, mut at) = head(protected, 0);
    assert_eq!(major, 5, "the protected header is a map");
    let mut alg = None;
    for _ in 0..pairs {
        let (key_major, key, next) = head(protected, at);
        assert_eq!(key_major, 0, "an integer label");
        if key == 1 {
            let (major, argument, _) = head(protected, next);
            let argument = i64::try_from(argument).expect("fits");
            alg = Some(if major == 0 { argument } else { -1 - argument });
        }
        at = skip(protected, next);
    }
    Parts {
        alg: alg.expect("a protected alg"),
        signature_at: start,
    }
}

/// The name of a COSE algorithm, as the corpus labels it.
pub(super) fn name_of(alg: i64) -> &'static str {
    match alg {
        -8 => "EdDSA",
        -7 => "ES256",
        -35 => "ES384",
        -36 => "ES512",
        other => panic!("{other} is not an algorithm of this crate"),
    }
}

/// The raw signature, `R ‖ S` for ECDSA.
pub(super) fn signature(message: &[u8]) -> Vec<u8> {
    message[parts(message).signature_at..].to_vec()
}

/// The same message with its signature replaced by one of the same length.
pub(super) fn with_signature(message: &[u8], signature: &[u8]) -> Vec<u8> {
    let at = parts(message).signature_at;
    assert_eq!(signature.len(), message.len() - at);
    [&message[..at], signature].concat()
}
