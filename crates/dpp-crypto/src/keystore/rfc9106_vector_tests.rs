//! RFC 9106 clause 5.3's Argon2id test vector, checked against the `argon2`
//! crate the keystore derives its keys with.
//!
//! The frozen derivation vectors in `tests.rs` were produced by this same crate,
//! at an earlier version. They pin that a derivation never changes under
//! existing stores, which is worth having, but they cannot show that the
//! function is Argon2id as RFC 9106 defines it: if the crate had always computed
//! something else, they would still hold. This vector is the RFC's.
//!
//! The keystore fixes its own cost parameters in `crypto.rs`, and they are not
//! the RFC's: the vector uses 32 KiB, three passes and four lanes, with a secret
//! and associated data that the keystore does not use. So the vector is run
//! through the crate directly, with the algorithm and version the keystore
//! selects and through the same entry point. That shows the crate implements the
//! function the keystore asks of it. The keystore's parameters remain its own
//! choice, held by `tests.rs`.

use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};

/// The RFC's tag: 32 bytes, printed in two lines.
const RFC_TAG: &str = "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659";

fn tag(password: &[u8; 32]) -> [u8; 32] {
    let params = ParamsBuilder::new()
        .m_cost(32)
        .t_cost(3)
        .p_cost(4)
        .output_len(32)
        .data(AssociatedData::new(&[0x04; 12]).expect("12 bytes of associated data"))
        .build()
        .expect("the RFC's parameters");
    let argon2 = Argon2::new_with_secret(&[0x03; 8], Algorithm::Argon2id, Version::V0x13, params)
        .expect("an 8-byte secret");

    let mut out = [0u8; 32];
    argon2
        .hash_password_into(password, &[0x02; 16], &mut out)
        .expect("a 16-byte salt and a 32-byte tag");
    out
}

#[test]
fn argon2id_reproduces_the_rfc_tag() {
    assert_eq!(hex::encode(tag(&[0x01; 32])), RFC_TAG);
}

/// The tag depends on the password: a vector that every input matched would
/// prove nothing.
#[test]
fn a_different_password_gives_a_different_tag() {
    assert_ne!(hex::encode(tag(&[0x02; 32])), RFC_TAG);
}
