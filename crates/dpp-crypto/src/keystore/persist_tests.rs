//! A save that fails leaves the handle as the file has it. Each save writes a temp
//! file beside the store and renames it over the store, so a directory the process
//! cannot write into fails every save. Unix only, since it relies on directory
//! permissions.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use crate::jws::algorithm::KeyAlgorithm;
use crate::keystore::KeyStore;

#[test]
fn a_failed_save_leaves_the_handle_and_the_file_as_they_were() {
    let dir = crate::test_support::temp_dir("keystore-readonly");
    std::fs::create_dir_all(&dir).expect("create the store's directory");
    let path = dir.join("keys.json");
    let store = KeyStore::open(&path, "test-passphrase").expect("open");
    let before = store.generate_key("issuer").expect("generate");

    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).expect("read-only");
    // A save writes into this directory, so the test means something only if this
    // process cannot write into it.
    assert!(
        std::fs::File::create(dir.join("probe")).is_err(),
        "the test needs a user who cannot write into a read-only directory"
    );
    let rotated = store.rotate_key_to("issuer", KeyAlgorithm::EcdsaP256);
    let generated = store.generate_key_with("other", KeyAlgorithm::EcdsaP256);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("restore");

    assert!(rotated.is_err(), "a rotation whose save fails must fail");
    assert!(
        generated.is_err(),
        "a generation whose save fails must fail"
    );
    let held = store
        .load_key("issuer")
        .expect("the handle still holds its key");
    assert_eq!(held.fingerprint, before.fingerprint);
    assert_eq!(held.algorithm(), KeyAlgorithm::Ed25519);
    assert!(
        !store.has_key("other"),
        "a key that was not saved is not held"
    );
    assert!(
        store.archived_public_keys("issuer").is_empty(),
        "nor is an archive that was not saved"
    );

    let reopened = KeyStore::open(&path, "test-passphrase").expect("reopen");
    assert_eq!(
        reopened.load_key("issuer").expect("load").fingerprint,
        before.fingerprint
    );
    assert!(!reopened.has_key("other"));
    let _ = std::fs::remove_dir_all(&dir);
}
