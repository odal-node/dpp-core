//! [`KeyEntry`] — a decrypted, in-memory key pair handed back by the store.

use zeroize::Zeroizing;

use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::public_key::PublicKey;

/// A decrypted key pair.
///
/// The private material is not public: it is reached only through
/// [`Self::sign`], so no caller can hold a curve-specific type and every
/// signature this crate makes goes through one per-algorithm `match`.
/// `algorithm` is the algorithm the key is recorded as using; it is what a
/// verifier binds the JWS `alg` header to. The struct is `#[non_exhaustive]` so
/// that adding an algorithm does not break downstream construction.
#[non_exhaustive]
pub struct KeyEntry {
    material: SigningMaterial,
    pub fingerprint: String,
    /// Whether this key has been revoked (see `KeyRecord::revoked`).
    pub revoked: bool,
    pub algorithm: KeyAlgorithm,
}

impl KeyEntry {
    pub(crate) fn new(material: SigningMaterial, fingerprint: String, revoked: bool) -> Self {
        let algorithm = material.algorithm();
        Self {
            material,
            fingerprint,
            revoked,
            algorithm,
        }
    }

    /// The public half, with its algorithm.
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        self.material.public_key()
    }

    /// Sign `message` under this key's algorithm and return the raw signature:
    /// 64 bytes for Ed25519.
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

/// The private half of a key pair, one variant per algorithm.
pub(crate) enum SigningMaterial {
    Ed25519(ed25519_dalek::SigningKey),
}

// The private key zeroizes when it drops. This fails to compile if the `zeroize`
// feature of ed25519-dalek is ever disabled, where a silent regression would not.
const _: () = {
    const fn assert_zeroize_on_drop<T: zeroize::ZeroizeOnDrop>() {}
    assert_zeroize_on_drop::<ed25519_dalek::SigningKey>();
};

impl SigningMaterial {
    /// A fresh key pair from the operating system's entropy source.
    pub(crate) fn generate(algorithm: KeyAlgorithm) -> Self {
        match algorithm {
            KeyAlgorithm::Ed25519 => {
                Self::Ed25519(ed25519_dalek::SigningKey::generate(&mut crate::os_rng()))
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
        }
    }

    /// The secret the store encrypts: the Ed25519 seed, 32 bytes.
    pub(crate) fn secret(&self) -> Zeroizing<Vec<u8>> {
        match self {
            Self::Ed25519(key) => Zeroizing::new(key.to_bytes().to_vec()),
        }
    }

    pub(crate) fn algorithm(&self) -> KeyAlgorithm {
        match self {
            Self::Ed25519(_) => KeyAlgorithm::Ed25519,
        }
    }

    pub(crate) fn public_key(&self) -> PublicKey {
        match self {
            // The public half of a valid signing key always decodes as a point,
            // so `from_bytes` cannot refuse it.
            Self::Ed25519(key) => {
                PublicKey::from_bytes(KeyAlgorithm::Ed25519, key.verifying_key().as_bytes())
                    .expect("the public half of a signing key is a valid public key")
            }
        }
    }

    pub(crate) fn sign(&self, message: &[u8]) -> Vec<u8> {
        match self {
            Self::Ed25519(key) => {
                use ed25519_dalek::Signer as _;
                key.sign(message).to_bytes().to_vec()
            }
        }
    }
}
