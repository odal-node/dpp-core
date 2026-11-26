//! One handle per store file: a second open is refused, in this process and in
//! another, and the lock goes when the handle does, or when the process ends.

use std::fs::TryLockError;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::super::error::KeyStoreLocked;
use super::super::lock::StoreLock;
use super::super::store::KeyStore;
use crate::test_support::{temp_dir, temp_path};

const PASSPHRASE: &str = "lock-test-passphrase";

/// `<store file>.lock` in the store's resolved directory, written out here and not
/// taken from the code under test.
fn sibling_lock(store: &Path) -> PathBuf {
    let dir = store
        .parent()
        .expect("a directory")
        .canonicalize()
        .expect("the directory exists");
    let name = store.file_name().expect("a file name").to_string_lossy();
    dir.join(format!("{name}.lock"))
}

/// The typed error a refused open carries. `KeyStore` has no `Debug`, since it holds a
/// passphrase-derived cipher, so a result is matched and not unwrapped.
fn refused(opened: anyhow::Result<KeyStore>) -> KeyStoreLocked {
    match opened {
        Ok(_) => panic!("a second handle on one file was opened"),
        Err(error) => error
            .downcast_ref::<KeyStoreLocked>()
            .cloned()
            .unwrap_or_else(|| panic!("refused, but not as KeyStoreLocked: {error:#}")),
    }
}

#[test]
fn a_second_open_of_a_file_is_refused_and_names_the_lock_file() {
    let path = temp_path("lock-second");
    let first = KeyStore::open(&path, PASSPHRASE).expect("open");
    let issuer = first.generate_key("issuer").expect("generate");
    let on_disk = std::fs::read(&path).expect("the store was written");

    let expected = KeyStoreLocked {
        lock_path: sibling_lock(&path),
    };
    // Both doors, and a passphrase that would fail anyway: the lock is the first thing
    // asked, so what is refused is the second handle and not what it carries.
    assert_eq!(refused(KeyStore::open(&path, PASSPHRASE)), expected);
    assert_eq!(
        refused(KeyStore::open_and_migrate(&path, PASSPHRASE)),
        expected
    );
    assert_eq!(refused(KeyStore::open(&path, "another")), expected);
    assert!(
        expected
            .to_string()
            .contains(&*expected.lock_path.to_string_lossy()),
        "the message names the lock file: {expected}"
    );

    // The handle that holds the lock is unharmed, and the file is as it was.
    assert_eq!(std::fs::read(&path).expect("read"), on_disk);
    assert_eq!(
        first.load_key("issuer").expect("load").fingerprint,
        issuer.fingerprint
    );
    first
        .generate_key("second-key")
        .expect("the holder still writes");

    drop(first);
    let again = KeyStore::open(&path, PASSPHRASE).expect("a reopen after the drop");
    assert!(again.has_key("issuer") && again.has_key("second-key"));
}

#[test]
fn the_lock_file_is_a_sibling_and_stays_after_the_handle_is_dropped() {
    let path = temp_path("lock-sibling");
    let lock = sibling_lock(&path);
    assert!(!lock.exists(), "the control: nothing before the first open");

    let store = KeyStore::open(&path, PASSPHRASE).expect("open");
    assert!(lock.exists(), "the open made it");
    let dir = path
        .parent()
        .expect("a directory")
        .canonicalize()
        .expect("the directory exists");
    assert_eq!(
        lock.parent(),
        Some(dir.as_path()),
        "in the store's directory"
    );
    assert!(
        lock.to_string_lossy().ends_with(".json.lock"),
        "named for the store file: {}",
        lock.display()
    );
    drop(store);
    assert!(lock.exists(), "dropping the handle does not remove it");
}

/// The control of the rest: the lock is on one file, and not on the process or the
/// directory.
#[test]
fn two_files_are_locked_independently() {
    let first = KeyStore::open(temp_path("lock-one"), PASSPHRASE).expect("open one");
    let second = KeyStore::open(temp_path("lock-two"), PASSPHRASE).expect("open another");
    first.generate_key("a").expect("generate");
    second.generate_key("b").expect("generate");
}

/// A relative path and an absolute one, or one through a `..`, are one file. Each would
/// lock its own file if the lock file were named from the path as it was spelt.
#[test]
fn two_spellings_of_one_path_are_one_lock() {
    let dir = temp_dir("lock-spelling");
    std::fs::create_dir_all(dir.join("sub")).expect("make the directories");
    let direct = dir.join("keys.json");
    let roundabout = dir.join("sub").join("..").join("keys.json");

    let first = KeyStore::open(&direct, PASSPHRASE).expect("open");
    let second = refused(KeyStore::open(&roundabout, PASSPHRASE));
    assert_eq!(second.lock_path, sibling_lock(&direct));
    drop(first);
    KeyStore::open(&roundabout, PASSPHRASE).expect("free once the first is dropped");
}

/// A symlink to a store file is that store, whichever name opens it first. The handle also
/// saves through the resolved path: a save renames a temp file over the store, which
/// would replace the link and leave the file it named behind.
#[cfg(unix)]
#[test]
fn a_symlink_to_a_store_file_is_one_store_with_the_file() {
    let dir = temp_dir("lock-symlink");
    std::fs::create_dir_all(&dir).expect("make the directory");
    let target = dir.join("keys.json");
    let link = dir.join("current.json");
    let store = KeyStore::open(&target, PASSPHRASE).expect("open");
    store.generate_key("issuer").expect("generate");
    drop(store);
    std::os::unix::fs::symlink(&target, &link).expect("make the link");

    // The file first: the link is refused, with the file's lock.
    let first = KeyStore::open(&target, PASSPHRASE).expect("open the file");
    assert_eq!(
        refused(KeyStore::open(&link, PASSPHRASE)).lock_path,
        sibling_lock(&target)
    );
    drop(first);

    // The link first: the file is refused, and a save keeps the link a link.
    let through_link = KeyStore::open(&link, PASSPHRASE).expect("open the link");
    assert_eq!(
        refused(KeyStore::open(&target, PASSPHRASE)).lock_path,
        sibling_lock(&target)
    );
    through_link.generate_key("added").expect("generate");
    assert!(
        std::fs::symlink_metadata(&link)
            .expect("the link is still there")
            .file_type()
            .is_symlink(),
        "a save through the link replaced the link"
    );
    drop(through_link);
    let reopened = KeyStore::open(&target, PASSPHRASE).expect("reopen the file");
    assert!(
        reopened.has_key("issuer") && reopened.has_key("added"),
        "the save went to the file the link names"
    );
}

/// A store file named like a lock file is refused, whichever store holds the lock. A store
/// saved over `keys.lock` would replace the lock file of the store `keys`, and leave that
/// store's lock on a file nothing opens, so a second handle on `keys` would be let in.
#[test]
fn a_store_file_cannot_be_named_like_a_lock_file() {
    let dir = temp_dir("lock-reserved");
    std::fs::create_dir_all(&dir).expect("make the directory");
    let keys = dir.join("keys");
    let first = KeyStore::open(&keys, PASSPHRASE).expect("open");

    let Err(refusal) = KeyStore::open(dir.join("keys.lock"), PASSPHRASE) else {
        panic!("a store was opened on another store's lock file");
    };
    assert!(
        refusal.downcast_ref::<KeyStoreLocked>().is_none(),
        "refused for its name, not for the lock: {refusal:#}"
    );
    assert!(
        format!("{refusal:#}").contains("cannot be a key store file"),
        "{refusal:#}"
    );
    assert!(
        !dir.join("keys.lock.lock").exists(),
        "the refused open made no lock file of its own"
    );

    // The lock on `keys` is intact, and a save does not disturb it.
    first.generate_key("issuer").expect("generate");
    assert_eq!(
        refused(KeyStore::open(&keys, PASSPHRASE)).lock_path,
        sibling_lock(&keys)
    );
    // The control: a name that merely contains `lock` is a store like any other.
    KeyStore::open(dir.join("lock.json"), PASSPHRASE).expect("a store named lock.json");
}

/// An open that fails after it has the lock lets go of it: a wrong passphrase must not
/// leave the file locked for the one that is right.
#[test]
fn an_open_that_fails_releases_the_lock() {
    let path = temp_path("lock-failed");
    let store = KeyStore::open(&path, PASSPHRASE).expect("open");
    store.generate_key("issuer").expect("generate");
    drop(store);

    let Err(wrong) = KeyStore::open(&path, "the wrong passphrase") else {
        panic!("a wrong passphrase opened the store");
    };
    assert!(
        wrong.downcast_ref::<KeyStoreLocked>().is_none(),
        "it failed on the passphrase, not on the lock: {wrong:#}"
    );
    KeyStore::open(&path, PASSPHRASE).expect("the failed open let go of the lock");
}

/// A platform that cannot lock refuses the open, since a store that is not locked is
/// the defect, and says why; an ordinary I/O failure is not mistaken for either.
#[test]
fn a_platform_that_cannot_lock_refuses_the_open() {
    let path = temp_path("lock-unsupported");

    let unsupported = StoreLock::acquire_with(&path, |_| {
        Err(TryLockError::Error(std::io::ErrorKind::Unsupported.into()))
    })
    .err()
    .expect("an unsupported lock is refused");
    assert!(unsupported.downcast_ref::<KeyStoreLocked>().is_none());
    assert!(
        format!("{unsupported:#}").contains("cannot lock"),
        "{unsupported:#}"
    );

    let failed = StoreLock::acquire_with(&path, |_| {
        Err(TryLockError::Error(
            std::io::ErrorKind::PermissionDenied.into(),
        ))
    })
    .err()
    .expect("a failed lock is refused");
    assert!(failed.downcast_ref::<KeyStoreLocked>().is_none());
    assert!(!format!("{failed:#}").contains("cannot lock"), "{failed:#}");

    // The control: a lock that is simply held is the typed refusal.
    let held = StoreLock::acquire_with(&path, |_| Err(TryLockError::WouldBlock))
        .err()
        .expect("a held lock is refused");
    assert_eq!(
        held.downcast_ref::<KeyStoreLocked>(),
        Some(&KeyStoreLocked {
            lock_path: sibling_lock(&path)
        })
    );
}

// ── Another process ───────────────────────────────────────────────────────────

/// The environment variable that tells the test below's other process which store to
/// hold.
const HOLDER_PATH: &str = "DPP_CRYPTO_KEYSTORE_LOCK_HOLDER";
/// What the other process prints once it holds the store.
const HELD: &str = "keystore-lock-held";

/// The other process of the test below, which runs this binary again with this test as
/// its only one. On its own, without the variable, it does nothing.
#[test]
fn hold_the_store_for_the_cross_process_test() {
    let Some(path) = std::env::var_os(HOLDER_PATH) else {
        return;
    };
    let _store = KeyStore::open(&path, PASSPHRASE).expect("the other process opens the store");
    println!("{HELD}");
    // Hold it until the parent kills this process, or ends and closes its stdin.
    let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
}

/// The refusal is across processes, which is the case the issue describes, and the lock
/// goes with a process that is killed, as one that crashes is.
#[test]
fn a_store_held_by_another_process_is_refused_until_that_process_ends() {
    let path = temp_path("lock-process");
    // libtest names a test without its crate: `keystore::tests::lock::name`.
    let (_crate, module) = module_path!().split_once("::").expect("a crate prefix");
    let mut other = Command::new(std::env::current_exe().expect("this test binary"))
        .args([
            "--exact",
            &format!("{module}::hold_the_store_for_the_cross_process_test"),
            "--nocapture",
        ])
        .env(HOLDER_PATH, &path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start the other process");

    let mut lines = BufReader::new(other.stdout.take().expect("piped")).lines();
    let held = lines.any(|line| line.is_ok_and(|line| line == HELD));
    assert!(held, "the other process never reported holding the store");

    assert_eq!(
        refused(KeyStore::open(&path, PASSPHRASE)).lock_path,
        sibling_lock(&path)
    );
    assert_eq!(
        refused(KeyStore::open_and_migrate(&path, PASSPHRASE)).lock_path,
        sibling_lock(&path)
    );

    // Killed, not asked to stop: nothing in it runs to release the lock.
    other.kill().expect("kill the other process");
    other.wait().expect("reap it");
    KeyStore::open(&path, PASSPHRASE).expect("the lock went with the process");
}
