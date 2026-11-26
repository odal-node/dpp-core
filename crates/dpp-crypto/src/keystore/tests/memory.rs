//! `MemoryKeyStore` outlives a thread that panicked while it held the lock.

use super::super::{KeyBackend, MemoryKeyStore};

/// The backend is exported behind a feature, so it is library code: a poisoned lock must
/// not turn every later call into a panic. Each method still answers, from the map as the
/// panicking thread left it, which is the map as it was.
#[test]
fn a_poisoned_lock_is_recovered_and_not_a_panic() {
    let store = MemoryKeyStore::new();
    let before = store.generate_key("k").expect("generate").public_key();

    store.poison_for_test();

    assert!(store.has_key("k"));
    assert_eq!(store.load_key("k").expect("load").public_key(), before);
    assert_eq!(
        store
            .public_key("k")
            .expect("recorded")
            .public_key()
            .expect("a key"),
        before
    );
    assert_eq!(store.list().expect("list").len(), 1);
    assert!(store.archived_public_keys("k").is_empty());
    store.rotate_key("k").expect("rotate after the poisoning");
    assert_eq!(store.archived_public_keys("k").len(), 1);
    store
        .generate_key("other")
        .expect("generate after the poisoning");
}
