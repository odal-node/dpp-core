//! [`KeyEntry`] — a decrypted, in-memory key pair handed back by the store.

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::public_key::PublicKey;

/// A decrypted key pair.
///
/// The private material is not public: it is reached only through
/// [`Self::sign`], so no caller can hold a curve-specific type and every
/// signature this crate makes goes through one per-algorithm `match`.
/// [`Self::algorithm`] is the algorithm the key signs under; it is what a verifier
/// binds the JWS `alg` header to. The struct is `#[non_exhaustive]` so that adding an
/// algorithm does not break downstream construction.
#[non_exhaustive]
pub struct KeyEntry {
    material: SigningMaterial,
    pub fingerprint: String,
    /// Whether this key has been revoked (see `KeyRecord::revoked`).
    pub revoked: bool,
}

impl KeyEntry {
    pub(crate) fn new(material: SigningMaterial, fingerprint: String, revoked: bool) -> Self {
        Self {
            material,
            fingerprint,
            revoked,
        }
    }

    /// The algorithm the key signs under, read from the key itself.
    #[must_use]
    pub fn algorithm(&self) -> KeyAlgorithm {
        self.material.algorithm()
    }

    /// The public half, with its algorithm.
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        self.material.public_key()
    }

    /// Sign `message` under this key's algorithm and return the raw signature:
    /// 64 bytes for Ed25519 and ES256, 96 for ES384 and 132 for ES512 (`R ‖ S`, low-`S`).
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.material.sign(message)
    }

    /// The secret, for tests that prove it never leaks into output or logs.
    #[cfg(test)]
    pub(crate) fn secret_for_test(&self) -> Zeroizing<Vec<u8>> {
        self.material.secret()
    }
}

/// The fingerprint a key pair is recorded under: the hex SHA-256 of its public bytes.
/// The store binds each record's ciphertext to it, so every path that makes a key
/// pair computes it this way.
pub(crate) fn fingerprint_of(public: &PublicKey) -> String {
    hex::encode(Sha256::digest(public.as_bytes()))
}

/// The private half of a key pair, one variant per algorithm.
pub(crate) enum SigningMaterial {
    Ed25519(ed25519_dalek::SigningKey),
    EcdsaP256(p256::ecdsa::SigningKey),
    EcdsaP384(p384::ecdsa::SigningKey),
    EcdsaP521(p521::ecdsa::SigningKey),
}

// The private keys zeroize when they drop. This fails to compile if the `zeroize`
// feature of ed25519-dalek, or the zeroizing impl of ecdsa, is ever lost, where a
// silent regression would not.
const _: () = {
    const fn assert_zeroize_on_drop<T: zeroize::ZeroizeOnDrop>() {}
    assert_zeroize_on_drop::<ed25519_dalek::SigningKey>();
    assert_zeroize_on_drop::<p256::ecdsa::SigningKey>();
    assert_zeroize_on_drop::<p384::ecdsa::SigningKey>();
    assert_zeroize_on_drop::<p521::ecdsa::SigningKey>();
};

impl SigningMaterial {
    /// A fresh key pair from the operating system's entropy source.
    pub(crate) fn generate(algorithm: KeyAlgorithm) -> Self {
        match algorithm {
            KeyAlgorithm::Ed25519 => {
                Self::Ed25519(ed25519_dalek::SigningKey::generate(&mut crate::os_rng()))
            }
            KeyAlgorithm::EcdsaP256 => {
                // A uniformly random scalar, retried in the rare case that it is not
                // below the group order: about 2^-32 of 256-bit values are at least `n`.
                // Zero is negligibly likely.
                loop {
                    let mut scalar = Zeroizing::new([0u8; 32]);
                    rand::Rng::fill_bytes(&mut crate::os_rng(), scalar.as_mut());
                    if let Ok(key) = p256::ecdsa::SigningKey::from_slice(scalar.as_ref()) {
                        break Self::EcdsaP256(key);
                    }
                }
            }
            KeyAlgorithm::EcdsaP384 => {
                use p384::elliptic_curve::Generate as _;
                // The curve's own sampler. Its 48 octets fill the order's 384 bits, so a
                // draw is refused only when it is at least `n`, about 2^-194 of the time.
                Self::EcdsaP384(p384::ecdsa::SigningKey::generate_from_rng(
                    &mut crate::os_rng(),
                ))
            }
            KeyAlgorithm::EcdsaP521 => {
                use p521::elliptic_curve::Generate as _;
                // The curve's own sampler. Its 66 octets hold 528 bits and the order `n` has
                // 521, so a draw is refused when it is at least `n`. `n` is `2^521` less about
                // `2^259`, so `n / 2^528` is 1/128 to within 2^-269, and the refusal probability
                // `1 − n / 2^528` is 127/128 to that same precision. The sampler draws again, so
                // a key takes about 128 draws of 66 octets on average.
                Self::EcdsaP521(p521::ecdsa::SigningKey::generate_from_rng(
                    &mut crate::os_rng(),
                ))
            }
        }
    }

    /// Rebuild a key pair from the secret the store encrypted.
    ///
    /// # Errors
    /// When the secret is not a valid private key of `algorithm`.
    pub(crate) fn from_secret(algorithm: KeyAlgorithm, secret: &[u8]) -> anyhow::Result<Self> {
        match algorithm {
            KeyAlgorithm::Ed25519 => {
                // The array is a second copy of the private key, so it is
                // `Zeroizing` too: it is cleared when it drops.
                let bytes: Zeroizing<[u8; 32]> = Zeroizing::new(
                    secret
                        .try_into()
                        .map_err(|_| anyhow::anyhow!("unexpected key length"))?,
                );
                Ok(Self::Ed25519(ed25519_dalek::SigningKey::from_bytes(&bytes)))
            }
            KeyAlgorithm::EcdsaP256 => p256::ecdsa::SigningKey::from_slice(secret)
                .map(Self::EcdsaP256)
                .map_err(|_| anyhow::anyhow!("not a P-256 private key")),
            KeyAlgorithm::EcdsaP384 => p384::ecdsa::SigningKey::from_slice(secret)
                .map(Self::EcdsaP384)
                .map_err(|_| anyhow::anyhow!("not a P-384 private key")),
            KeyAlgorithm::EcdsaP521 => p521::ecdsa::SigningKey::from_slice(secret)
                .map(Self::EcdsaP521)
                .map_err(|_| anyhow::anyhow!("not a P-521 private key")),
        }
    }

    /// The secret the store encrypts: the Ed25519 seed (32 octets), or an EC scalar
    /// of the curve's field size (32, 48 or 66 octets for P-256, P-384 and P-521).
    pub(crate) fn secret(&self) -> Zeroizing<Vec<u8>> {
        match self {
            // Copied straight from the key's borrowed secret, so no unprotected array is
            // made on the way. The copy is cleared on drop.
            Self::Ed25519(key) => Zeroizing::new(key.as_bytes().to_vec()),
            // The scalar comes back by value, so it is wrapped in `Zeroizing` at once and
            // cleared when it drops. The copy the caller gets is cleared on drop too.
            Self::EcdsaP256(key) => {
                let scalar = Zeroizing::new(key.to_bytes());
                Zeroizing::new(scalar.to_vec())
            }
            Self::EcdsaP384(key) => {
                let scalar = Zeroizing::new(key.to_bytes());
                Zeroizing::new(scalar.to_vec())
            }
            Self::EcdsaP521(key) => {
                let scalar = Zeroizing::new(key.to_bytes());
                Zeroizing::new(scalar.to_vec())
            }
        }
    }

    pub(crate) fn algorithm(&self) -> KeyAlgorithm {
        match self {
            Self::Ed25519(_) => KeyAlgorithm::Ed25519,
            Self::EcdsaP256(_) => KeyAlgorithm::EcdsaP256,
            Self::EcdsaP384(_) => KeyAlgorithm::EcdsaP384,
            Self::EcdsaP521(_) => KeyAlgorithm::EcdsaP521,
        }
    }

    pub(crate) fn public_key(&self) -> PublicKey {
        match self {
            Self::Ed25519(key) => PublicKey::from_ed25519_key(key.verifying_key()),
            Self::EcdsaP256(key) => PublicKey::from_p256_key(*key.verifying_key()),
            Self::EcdsaP384(key) => PublicKey::from_p384_key(*key.verifying_key()),
            Self::EcdsaP521(key) => PublicKey::from_p521_key(*key.verifying_key()),
        }
    }

    pub(crate) fn sign(&self, message: &[u8]) -> Vec<u8> {
        match self {
            Self::Ed25519(key) => {
                use ed25519_dalek::Signer as _;
                key.sign(message).to_bytes().to_vec()
            }
            Self::EcdsaP256(key) => {
                use p256::ecdsa::signature::Signer as _;
                // RFC 6979 deterministic nonce, SHA-256. Normalised to low-S so
                // this crate's output is canonical; verification accepts both.
                let signature: p256::ecdsa::Signature = key.sign(message);
                signature.normalize_s().to_bytes().to_vec()
            }
            Self::EcdsaP384(key) => {
                use p384::ecdsa::signature::Signer as _;
                // RFC 6979 deterministic nonce, SHA-384. Normalised to low-S so this
                // crate's output is canonical; verification accepts both.
                let signature: p384::ecdsa::Signature = key.sign(message);
                signature.normalize_s().to_bytes().to_vec()
            }
            Self::EcdsaP521(key) => {
                use p521::ecdsa::signature::Signer as _;
                // RFC 6979 deterministic nonce, SHA-512. Normalised to low-S so this
                // crate's output is canonical; verification accepts both.
                let signature: p521::ecdsa::Signature = key.sign(message);
                signature.normalize_s().to_bytes().to_vec()
            }
        }
    }
}
