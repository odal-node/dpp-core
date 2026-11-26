//! Helpers the signature tests share.

use serde_json::{Value, json};

use super::super::algorithm::KeyAlgorithm;
use super::super::public_key::PublicKey;

pub(super) const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

pub(super) fn p256(jwk: &Value) -> PublicKey {
    PublicKey::from_jwk(jwk).expect("a P-256 JWK")
}

pub(super) fn rfc7515_key() -> PublicKey {
    p256(&json!({
        "kty": "EC",
        "crv": "P-256",
        "x": "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU",
        "y": "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0",
    }))
}

/// `n / 2`, rounded down: an `S` above it is "high".
pub(super) const HALF_P384: [u8; 48] = hex_bytes(
    "7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE3B1A6C0FA1B96EFAC0D06D9245853BD76760CB5666294B9",
);

/// `n / 2`, rounded down, in 66 octets.
pub(super) const HALF_P521: [u8; 66] = hex_bytes(
    "00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFD28C343C1DF97CB35BFE600A47B84D2E81DDAE4DC44CE23D75DB7DB8F489C3204",
);

/// The affine coordinates of RFC 6979 clause A.2.6's P-384 public key, `Ux` and `Uy`.
pub(super) const RFC6979_P384_UX: &str = "EC3A4E415B4E19A4568618029F427FA5DA9A8BC4AE92E02E06AAE5286B300C64DEF8F0EA9055866064A254515480BC13";

pub(super) const RFC6979_P384_UY: &str = "8015D9B72D7D57244EA8EF9AC0C621896708A59367F9DFB9F54CA84B3F1C9DB1288B231C3AE0D4FE7344FD2533264720";

/// Whether `signature` (`R ‖ S`, `N` octets each) has an `S` above `half`.
pub(super) fn is_high_s<const N: usize>(half: &[u8; N], signature: &[u8]) -> bool {
    signature[N..] > half[..]
}

/// The P-384 public key of RFC 6979 clause A.2.6, read from its published point.
pub(super) fn rfc6979_p384_key() -> PublicKey {
    let mut sec1 = vec![0x04];
    sec1.extend_from_slice(&hex::decode(RFC6979_P384_UX).expect("hex"));
    sec1.extend_from_slice(&hex::decode(RFC6979_P384_UY).expect("hex"));
    PublicKey::from_bytes(KeyAlgorithm::EcdsaP384, &sec1).expect("the RFC's P-384 point")
}

/// The P-521 public key of RFC 7515 Appendix A.4, read from its published `x` and `y`.
pub(super) fn rfc7515_p521_key() -> PublicKey {
    PublicKey::from_jwk(&json!({
        "kty": "EC",
        "crv": "P-521",
        "x": "AekpBQ8ST8a8VcfVOTNl353vSrDCLLJXmPk06wTjxrrjcBpXp5EOnYG_NjFZ6OvLFV1jSfS9tsz4qUxcWceqwQGk",
        "y": "ADSmRA43Z1DSNx_RvcLI87cdL07l6jQyyBXMoxVg_l2Th-x3S1WDhjDly79ajL4Kkd0AZMaZmh9ubmf63e3kyMj2",
    }))
    .expect("the RFC's P-521 point")
}

/// One of clause 7.1's tests. Everything is lowercase hexadecimal.
pub(super) struct Vector {
    pub(super) name: &'static str,
    pub(super) secret: &'static str,
    pub(super) public: &'static str,
    pub(super) message: String,
    pub(super) signature: &'static str,
}

/// The 1023-byte message of the clause's fourth test, in the RFC's line breaks.
pub(super) const MESSAGE_1023: [&str; 64] = [
    "08b8b2b733424243760fe426a4b54908",
    "632110a66c2f6591eabd3345e3e4eb98",
    "fa6e264bf09efe12ee50f8f54e9f77b1",
    "e355f6c50544e23fb1433ddf73be84d8",
    "79de7c0046dc4996d9e773f4bc9efe57",
    "38829adb26c81b37c93a1b270b20329d",
    "658675fc6ea534e0810a4432826bf58c",
    "941efb65d57a338bbd2e26640f89ffbc",
    "1a858efcb8550ee3a5e1998bd177e93a",
    "7363c344fe6b199ee5d02e82d522c4fe",
    "ba15452f80288a821a579116ec6dad2b",
    "3b310da903401aa62100ab5d1a36553e",
    "06203b33890cc9b832f79ef80560ccb9",
    "a39ce767967ed628c6ad573cb116dbef",
    "efd75499da96bd68a8a97b928a8bbc10",
    "3b6621fcde2beca1231d206be6cd9ec7",
    "aff6f6c94fcd7204ed3455c68c83f4a4",
    "1da4af2b74ef5c53f1d8ac70bdcb7ed1",
    "85ce81bd84359d44254d95629e9855a9",
    "4a7c1958d1f8ada5d0532ed8a5aa3fb2",
    "d17ba70eb6248e594e1a2297acbbb39d",
    "502f1a8c6eb6f1ce22b3de1a1f40cc24",
    "554119a831a9aad6079cad88425de6bd",
    "e1a9187ebb6092cf67bf2b13fd65f270",
    "88d78b7e883c8759d2c4f5c65adb7553",
    "878ad575f9fad878e80a0c9ba63bcbcc",
    "2732e69485bbc9c90bfbd62481d9089b",
    "eccf80cfe2df16a2cf65bd92dd597b07",
    "07e0917af48bbb75fed413d238f5555a",
    "7a569d80c3414a8d0859dc65a46128ba",
    "b27af87a71314f318c782b23ebfe808b",
    "82b0ce26401d2e22f04d83d1255dc51a",
    "ddd3b75a2b1ae0784504df543af8969b",
    "e3ea7082ff7fc9888c144da2af58429e",
    "c96031dbcad3dad9af0dcbaaaf268cb8",
    "fcffead94f3c7ca495e056a9b47acdb7",
    "51fb73e666c6c655ade8297297d07ad1",
    "ba5e43f1bca32301651339e22904cc8c",
    "42f58c30c04aafdb038dda0847dd988d",
    "cda6f3bfd15c4b4c4525004aa06eeff8",
    "ca61783aacec57fb3d1f92b0fe2fd1a8",
    "5f6724517b65e614ad6808d6f6ee34df",
    "f7310fdc82aebfd904b01e1dc54b2927",
    "094b2db68d6f903b68401adebf5a7e08",
    "d78ff4ef5d63653a65040cf9bfd4aca7",
    "984a74d37145986780fc0b16ac451649",
    "de6188a7dbdf191f64b5fc5e2ab47b57",
    "f7f7276cd419c17a3ca8e1b939ae49e4",
    "88acba6b965610b5480109c8b17b80e1",
    "b7b750dfc7598d5d5011fd2dcc5600a3",
    "2ef5b52a1ecc820e308aa342721aac09",
    "43bf6686b64b2579376504ccc493d97e",
    "6aed3fb0f9cd71a43dd497f01f17c0e2",
    "cb3797aa2a2f256656168e6c496afc5f",
    "b93246f6b1116398a346f1a641f3b041",
    "e989f7914f90cc2c7fff357876e506b5",
    "0d334ba77c225bc307ba537152f3f161",
    "0e4eafe595f6d9d90d11faa933a15ef1",
    "369546868a7f3a45a96768d40fd9d034",
    "12c091c6315cf4fde7cb68606937380d",
    "b2eaaa707b4c4185c32eddcdd306705e",
    "4dc1ffc872eeee475a64dfac86aba41c",
    "0618983f8741c5ef68d3a101e8a3b8ca",
    "c60c905c15fc910840b94c00a0b9d0",
];

pub(super) fn vectors() -> Vec<Vector> {
    vec![
        Vector {
            name: "TEST 1",
            secret: "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
            public: "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            message: String::new(),
            signature: "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        },
        Vector {
            name: "TEST 2",
            secret: "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
            public: "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
            message: "72".to_owned(),
            signature: "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        },
        Vector {
            name: "TEST 3",
            secret: "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
            public: "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
            message: "af82".to_owned(),
            signature: "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
        },
        Vector {
            name: "TEST 1024",
            secret: "f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5",
            public: "278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e",
            message: MESSAGE_1023.concat(),
            signature: "0aab4c900501b3e24d7cdf4663326a3a87df5e4843b2cbdb67cbf6e460fec350aa5371b1508f9f4528ecea23c436d94b5e8fcd4f681e30a6ac00a9704a188a03",
        },
        Vector {
            name: "TEST SHA(abc)",
            secret: "833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42",
            public: "ec172b93ad5e563bf4932c70e1245034c35467ef2efd4d64ebf819683467e2bf",
            message: "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f".to_owned(),
            signature: "dc2a4459e7369633a52b1bf277839a00201009a3efbf3ecb69bea2186c26b58909351fc9ac90b3ecfdfbc7c66431e0303dca179c138ac17ad9bef1177331a704",
        },
    ]
}

/// The public key of one of clause 7.1's tests, as lowercase hexadecimal, for the
/// other test modules to reuse.
pub(super) fn public_hex(name: &str) -> &'static str {
    vectors()
        .iter()
        .find(|vector| vector.name == name)
        .map(|vector| vector.public)
        .expect("a test of RFC 8032 clause 7.1")
}

pub(super) const fn hex_bytes<const N: usize>(hex: &str) -> [u8; N] {
    let bytes = hex.as_bytes();
    assert!(bytes.len() == 2 * N, "one hex pair per octet");
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = nibble(bytes[2 * i]) << 4 | nibble(bytes[2 * i + 1]);
        i += 1;
    }
    out
}

pub(super) const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'A'..=b'F' => c - b'A' + 10,
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("not hex"),
    }
}
