//! The vendored test-suite inputs are the bytes the suite published, and nothing else
//! is in the directory.
//!
//! The files under `fixtures/w3c-vc-jose-cose` are the input files of the W3C
//! VC-JOSE-COSE test suite at commit `cf4c93ddf7e1bfef5a3e5db4b9a798b02ea778a9`:
//! 31 credentials, presentations and tokens, and the five verification-method files
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

use crate::test_support::{FILES, file};

/// File name, Git blob at the suite commit (empty for the licence, which is not a
/// suite blob), SHA-256.
const PINS: [(&str, &str, &str); 37] = [
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
        "credential-selective.json",
        "8a72f043fe40c0255bffdd0b5cc7dcafa354cb29",
        "5c201ed1ab5216f3ae8eee412dd90008aacf33657bcded7378f0b77f7d68cf82",
    ),
    (
        "credential-nested-selective.json",
        "99bc1e5b03d7707cdc82b452697626b00d076141",
        "458168beb99165327c2fa5d6bc926878f50ebf8c583c32166647cc3aa33b17ac",
    ),
    (
        "presentation-selective.json",
        "013ffa8f636175e39726e478c794ab649ae39fa7",
        "7d6ea237f42bdf5c70e1a6148ffb24ee8181606a50ec14d8ac1c141addc0880f",
    ),
    (
        "credential-sdjwt-selective.txt",
        "81b868ffd70b084eec84ef6eaa1e73f696ca7741",
        "a10dbe3263cca7537e99ffcc9afed82fdfc8513566f09ab601ff9f2d8fbdc8bf",
    ),
    (
        "credential-sdjwt-nested.txt",
        "cc21213d9ea93b8f72ca15a1dedda594980fd42f",
        "6007e8c7a6f5e8eb422364119f09d323b1ad77552fef2604c6c6141c99c19d8f",
    ),
    (
        "presentation-sdjwt-selective.txt",
        "1cc3a0a7a6ebbd8bb89b2549eae38887a3276fd4",
        "7c37549c25a8796bae7463a97755d9c74c1921d256c98dd280eb0d238a96c7f4",
    ),
    (
        "credential-sdjwt-bad-signature.txt",
        "e01b0b1c7035ebdf8573e024f330b67ac9fb5409",
        "b0af77fd5d7c72f5e5e7f75cc02af266ecb1705e1ee38a8b95e9dd08b26f1637",
    ),
    (
        "credential-sdjwt-bad-media-type.txt",
        "e04ad4782ff2e484f8559524b7628baa14cef842",
        "bc0fc5631a712059a83f0619d0fc4e61683c87ed18e560c30b5862a7795a2821",
    ),
    (
        "presentation-sdjwt-bad-media-type.txt",
        "b4a6ced238c127f92e926327af049633cdd0d0d2",
        "56f30bc449f1a124b587698603fb290ac00efcde5e120617b124dfa714b73b89",
    ),
    (
        "presentation-sdjwt-bad-credential.txt",
        "f43e1573c201e9c7f68df24eaef1264cdde7ec0c",
        "eb21b918a0320a5ac76812200b0a980e4ab5c4a31cceceb74755c5adbb52384c",
    ),
    (
        "credential-cose-minimal.txt",
        "b37ea9311dc5a1539b6a24e6aa7f2e1d148e17c6",
        "9fcf0feca6d3a7b4f30c0ea05b9c28ca937c5c85804d535f9ba26db187898858",
    ),
    (
        "credential-cose-minimal-not-base64.txt",
        "04e8ecd8ed904ab37027d4f8cb3764582224e871",
        "c72388c39456851a4010d2ab084bebda6dd9e2d0f18deec599bebc45dfe0faf1",
    ),
    (
        "credential-cose-bad-signature.txt",
        "b3d483a4fb649edb102d747937758961076c3821",
        "a639313a3fc839af7db9fdd7e1eac8f1f52ec4fefba142f937d3d5589c8c8b32",
    ),
    (
        "credential-cose-bad-media-type.txt",
        "d2306428d0c3d9105878ed2b6d0fafecc3ec4638",
        "c273604f83befa1d371483a07aea3c064095618e4215a910ffd653ae361a06bf",
    ),
    (
        "presentation-cose-single.txt",
        "e02ce4ae01bb352bdc9912d72ef735aafaeffd11",
        "3e38c4289a9da1df1e7c9025207521d8199b351970ba72e12d7985d004c2e250",
    ),
    (
        "presentation-cose-bad-credential.txt",
        "321cc78cf237773c1531efe7d2405537219fd14a",
        "bac192c09d8374b01111fece892a5e2b1d3fd3b4c5fe00b053187d5f532bee9f",
    ),
    (
        "presentation-cose-bad-media-type.txt",
        "7cf07edc39ff137e1ef49e5b384964aac4f8b6ad",
        "a3022908887351988ef744dbd019dbe7d6a1112afe5477d40c5ca2b67c3cdd41",
    ),
    (
        "vm-did-ed25519.json",
        "301cc78ea0d8b61418197328460b8f56f4928459",
        "c27e23c7bd5c1f9429bf7386765868f5b43fb36da88d1e7b4607fc220f006606",
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
