//! The vendored test-suite inputs are the bytes the suite published, and nothing else
//! is in the directory.
//!
//! The files under `fixtures/w3c-vc-jose-cose` are the input files of the W3C
//! VC-JOSE-COSE test suite at commit `cf4c93ddf7e1bfef5a3e5db4b9a798b02ea778a9`:
//! 14 credentials, presentations and tokens, and the four verification-method files
//! that carry the suite's keys. The suite is not ours. Its files are used under the
//! W3C Software and Document License, whose text, and the suite's statement of it,
//! is `LICENSE.md` beside them.
//!
//! Each file's SHA-256 is pinned below, and so is the Git blob it is at that commit
//! (recorded for provenance; the test computes the SHA-256). A changed byte fails
//! here, so a suite result in `suite_tests` is always a result on the suite's input.
//! `.gitattributes` marks the directory `-text`, so a checkout that normalises line
//! endings cannot change the bytes under the pin.

use sha2::{Digest, Sha256};

use super::test_fixtures::{FILES, file};

/// File name, Git blob at the suite commit (empty for the licence, which is not a
/// suite blob), SHA-256.
const PINS: [(&str, &str, &str); 19] = [
    (
        "credential-minimal.json",
        "6433f8ba94c018de1978f3251e9133356b32e156",
        "ab6a7f8e37dbf2359781887d0008043a37e766d139945c9cd02f979f75ad75fc",
    ),
    (
        "credential-full.json",
        "bb009fddf1ecc1374a58e5c8cb2b3c06795c87c3",
        "6db2b3f8d7470cffa99866ab46905cf52c4638602f3abac2e64caca61a8cdaa0",
    ),
    (
        "presentation-single.json",
        "33b6f4722de17386c592aacc44a4c752d995694b",
        "43465c171d187ba0567025af1277ebb4e9b437472af4b60698e8f4fcb6d5a9c3",
    ),
    (
        "presentation-multiple.json",
        "c3a03da343b712c4e2de5305b30d7f468d9a14ab",
        "dd83ba16ff8a63d811210f1d021fac817a4b42ff16eb8c1879737b71aed974a6",
    ),
    (
        "credential-unknown-extensions.json",
        "244491e5d34d445bd3ee8bbc06cb5f61a423c905",
        "4c5a2b43a2daa495c8a43ee09927b462fceaf3219be43fba3d9fcc2a2c40344b",
    ),
    (
        "credential-jose-minimal.txt",
        "e82f0c3ce43719575489303ec4d005e979ae2086",
        "5ba5ad68786bfbeeed5a49b6014d3bb1ba4023e37e48986605997fc7b70fe1d4",
    ),
    (
        "presentation-jose-multiple.txt",
        "a55d11553c9f6171dd41a87c4a74c8a4aee346b5",
        "4e502911864773a07cfcd7e07caf9280aa8a5315198e5fbabb0d8e760e01e1c3",
    ),
    (
        "credential-issuer-match-signed.txt",
        "bc1c0c715b7b68488d24e9f8ec4547086a690c43",
        "61d2a59ea51c378785674775f7017a4c20cdeaf4c96de59de1b0413adf836918",
    ),
    (
        "credential-jose-unknown-extensions.txt",
        "22c600ae158640fc2ccb7fd6778c6772142c6653",
        "ca372352265f51bd6fdcd46d887dd971c0fcb0257685375830ea88c6a3369416",
    ),
    (
        "credential-jose-bad-signature.txt",
        "6f304bae8504e38893e5346e76d76ae7c63c7860",
        "61bebb3d60c7e33684281075848b28ea45eb3f878335d547704a79b8aee8d268",
    ),
    (
        "credential-jose-bad-media-type.txt",
        "a40ec8b6e530317679b18605ba472cbf189c979e",
        "41f9986dbaf0aa42c7e44102af4670ef0808839c007e986fa5f174d31e398fd2",
    ),
    (
        "presentation-jose-bad-media-type.txt",
        "1ced3a489ca7c471d8a1fdff8e2aa2705912d698",
        "12e4d2bfcbac30fb3f7c8d13a8d92af89e2d0d1a662e01b21d287768561a6b48",
    ),
    (
        "credential-jose-vc-vp-claims.txt",
        "904c7bc9915744e7f91ac55b84455016b2ce865a",
        "54144e380c5f57f5b51e1d40477161787609e2188029161cd82aee0964102228",
    ),
    (
        "presentation-jose-bad-credential.txt",
        "15cad99e3b95df1914f59c2201e770951ac317de",
        "1c43bb7b349f6cf9b10f783a23ca97fa1b740bfe9bc95193bbe754c985de512f",
    ),
    (
        "vm-ed25519.json",
        "658e6a01446123c7380834aadc44243c9df0b534",
        "f4f8a3ea7575a1e0d60afb2a46fc2b07a60a7b0cfffdeb554f001f36b34c0c67",
    ),
    (
        "vm-p256.json",
        "b89c663eb2ff96c440f36503c71248d8ebc831e5",
        "7545ad74e80bd72c772c5ff6ef2ac9bc4ac33a0f345906f7cd769268ebee9ff1",
    ),
    (
        "vm-p384.json",
        "09c22e30af7d67a86ff3e00e5a1c1ff3cf7cdaf6",
        "b4ad48326ccebfb2a945fffab02920447698fff0b39e15fdc6ec0a87b657e919",
    ),
    (
        "vm-p521.json",
        "aac5c45870df4d4681b71f0df6efbb65cd7c0990",
        "61760b4b4bafe49d68b6780acae3a0fd9e264959b8327b3f072e78a757a9ab3a",
    ),
    (
        "LICENSE.md",
        "",
        "e04c7f00e2a0e09909c151639fe4c47a8fe03ba37f87fd5f10799d6544bf8e67",
    ),
];

#[test]
fn every_vendored_file_is_pinned_and_matches_its_pin() {
    assert_eq!(FILES.len(), PINS.len());
    for (name, _blob, sha256) in PINS {
        let digest = Sha256::digest(file(name).as_bytes());
        assert_eq!(hex::encode(digest), sha256, "{name}");
    }
}

/// The directory holds exactly the files that are pinned: a file added without a pin
/// fails here instead of going unchecked.
#[test]
fn the_directory_holds_only_the_pinned_files() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/jose/fixtures/w3c-vc-jose-cose");
    let mut on_disk: Vec<String> = std::fs::read_dir(&directory)
        .expect("the fixture directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    on_disk.sort();
    let mut pinned: Vec<String> = PINS.iter().map(|(name, ..)| (*name).to_owned()).collect();
    pinned.sort();
    assert_eq!(on_disk, pinned);
}

/// The suite's tokens end in a newline in some files and not in others, and carry no
/// carriage return in any. A carriage return would be what a checkout that
/// translates line endings leaves behind.
#[test]
fn the_tokens_carry_no_carriage_return() {
    for (name, text) in FILES {
        if name.ends_with(".txt") {
            assert!(!text.contains('\r'), "{name}");
        }
    }
}
