//! The vendored test-suite inputs are the bytes the suite published, and nothing else
//! is in the directory.
//!
//! The files under `fixtures` are COSE input files of the W3C VC-JOSE-COSE test
//! suite at commit `cf4c93ddf7e1bfef5a3e5db4b9a798b02ea778a9`: seven credentials and
//! presentations, and the four key files they are verified under. The suite is not
//! ours. Its files are used under the W3C Software and Document License, whose text,
//! and the suite's statement of it, is `LICENSE.md` beside them.
//!
//! Each file's SHA-256 is pinned below, and so is the Git blob it is at that commit
//! (recorded for provenance; the test computes the SHA-256). A changed byte fails
//! here, so a suite result in `suite_tests` is always a result on the suite's input.
//! `.gitattributes` marks the directory `-text`, so a checkout that normalises line
//! endings cannot change the bytes under the pin.

use sha2::{Digest, Sha256};

use super::super::test_support::{FILES, file};

/// File name, Git blob at the suite commit (empty for the licence, which is not a
/// suite blob), SHA-256.
const PINS: [(&str, &str, &str); 12] = [
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
        "vm-ed25519.json",
        "658e6a01446123c7380834aadc44243c9df0b534",
        "f4f8a3ea7575a1e0d60afb2a46fc2b07a60a7b0cfffdeb554f001f36b34c0c67",
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
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cose/fixtures");
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
