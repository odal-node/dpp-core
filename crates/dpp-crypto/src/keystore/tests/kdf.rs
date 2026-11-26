//! The Argon2id key derivation against its known answer.

/// Argon2id derivation is pinned to a known answer, so an upgrade of the
/// `argon2` crate cannot silently change it.
///
/// Every other keystore test derives a key and reads it back inside one
/// process, using the same `argon2` for both halves. A change in the KDF's
/// output would leave all of them green while making every store already on
/// disk unopenable — the one failure mode a round trip structurally cannot
/// see, and the reason this test is a fixture rather than another round trip.
///
/// The vectors were produced under `argon2` 0.5.3, the version in use while
/// the V2/V3 keystore format was written, so they are the bytes existing
/// stores were actually sealed with. Argon2id with explicit parameters is
/// fully specified (RFC 9106), so any correct implementation must reproduce
/// them. A difference here is either a real incompatibility or a deliberate
/// format change needing a keystore version bump and a migration path — never
/// something to re-baseline in place.
///
/// Parameters are fixed by `crypto.rs`: Argon2id, version 0x13, m=19456, t=2,
/// p=1, with the integrity key taking bytes 32..64 of a 64-byte output.
#[test]
fn argon2id_derivation_matches_its_known_answer() {
    let salt: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f,
    ];
    let passphrase = "correct horse battery staple";

    let aes =
        super::super::crypto::derive_aes_key_argon2(passphrase, &salt).expect("derive aes key");
    assert_eq!(
        hex::encode(aes),
        "818259b6310026a8e0dbac5d2e6927abcfdb07b32258fac4f61b18b80f929085",
        "Argon2id AES key derivation changed — every existing keystore is now unopenable"
    );

    let integrity = super::super::crypto::derive_integrity_key(passphrase, &salt)
        .expect("derive integrity key");
    assert_eq!(
        hex::encode(integrity),
        "420feff9f780f2b13b889f7a6dcf263da5a4310898aa51ff4de932e6a0adcd11",
        "Argon2id integrity key derivation changed — every existing keystore fails its HMAC"
    );
}
