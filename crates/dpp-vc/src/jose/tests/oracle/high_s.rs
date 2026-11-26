//! What the order of a NIST curve says about the `S` of an ECDSA signature.
//!
//! An `S` and `n − S` both verify, so a signer may write either. This crate writes
//! the lower one and verifies both (`PublicKey::verify`); Node's ECDSA does not
//! normalise, so about half of what it signs has the higher one. The orders are
//! the curves' own, FIPS 186-5 appendix D.1.2 (and SEC 2 §2.4.2, §2.5.2 and
//! §2.6.1); a wrong digit would make the twin of a signature fail to verify, which
//! `reverse` asserts.

/// The group order `n` of each curve, big-endian, in the octets of a coordinate.
const ORDERS: [(&str, &str); 3] = [
    (
        "ES256",
        "FFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551",
    ),
    (
        "ES384",
        "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFC7634D81F4372DDF581A0DB248B0A77AECEC196ACCC52973",
    ),
    (
        "ES512",
        "01FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA51868783BF2F966B7FCC0148F709A5D03BB5C9B8899C47AEBB6FB71E91386409",
    ),
];

/// Whether `alg` is one of the ECDSA algorithms.
pub(super) fn is_ecdsa(alg: &str) -> bool {
    ORDERS.iter().any(|(name, _)| *name == alg)
}

fn order(alg: &str) -> Vec<u8> {
    let (_, hex) = ORDERS
        .iter()
        .find(|(name, _)| *name == alg)
        .unwrap_or_else(|| panic!("{alg} is not an ECDSA algorithm"));
    hex::decode(hex).expect("hex")
}

/// `n / 2`, rounded down: an `S` above it is "high".
fn half(order: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; order.len()];
    let mut carry = 0u8;
    for (o, i) in out.iter_mut().zip(order) {
        *o = (carry << 7) | (i >> 1);
        carry = i & 1;
    }
    out
}

/// Whether `signature` (`R ‖ S`) has an `S` above `n / 2`.
pub(super) fn is_high_s(alg: &str, signature: &[u8]) -> bool {
    let order = order(alg);
    assert_eq!(signature.len(), 2 * order.len(), "R ‖ S of {alg}");
    signature[order.len()..] > half(&order)[..]
}

/// `R ‖ (n − S)`: the other valid signature over the same message.
pub(super) fn twin(alg: &str, signature: &[u8]) -> Vec<u8> {
    let order = order(alg);
    let width = order.len();
    assert_eq!(signature.len(), 2 * width, "R ‖ S of {alg}");
    let mut out = signature[..width].to_vec();
    let s = &signature[width..];
    let mut negated = vec![0u8; width];
    let mut borrow = 0i16;
    for i in (0..width).rev() {
        let mut d = i16::from(order[i]) - i16::from(s[i]) - borrow;
        borrow = 0;
        if d < 0 {
            d += 256;
            borrow = 1;
        }
        negated[i] = u8::try_from(d).expect("a byte");
    }
    out.extend_from_slice(&negated);
    out
}
