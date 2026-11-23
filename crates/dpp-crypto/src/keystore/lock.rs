//! The lock that gives a key store file one writer.
//!
//! It is taken on a **sibling file**, `<store file>.lock`, and not on the store
//! file: every write replaces the store by renaming a temp file over it, so a lock on
//! the old file would guard nothing once the first write had happened. The lock file
//! is never renamed or removed, so every handle that opens the store locks the same
//! file.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::error::KeyStoreLocked;

/// An exclusive lock on a store's lock file, held until it is dropped.
///
/// Dropping it closes the file, which releases the lock; so does the process ending,
/// however it ends.
pub(crate) struct StoreLock {
    /// Kept open for the lock's sake and never read.
    _file: File,
}

impl StoreLock {
    /// Take the lock for the store at `store_path`, creating the lock file if it is
    /// not there.
    ///
    /// Does not wait. A lock that is held is refused as [`KeyStoreLocked`].
    ///
    /// # Errors
    ///
    /// [`KeyStoreLocked`] when another handle holds the lock. Otherwise the lock file
    /// could not be made or opened, or the file system cannot lock files: an unlocked
    /// store is the defect this lock exists to prevent, so that refuses the open too.
    pub(crate) fn acquire(store_path: &Path) -> Result<Self> {
        Self::acquire_with(store_path, File::try_lock)
    }

    /// [`Self::acquire`], with the locking call supplied, so a test can say what a
    /// platform that cannot lock answers.
    pub(crate) fn acquire_with(
        store_path: &Path,
        try_lock: impl FnOnce(&File) -> Result<(), TryLockError>,
    ) -> Result<Self> {
        let lock_path = lock_path(store_path)?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("open the key store lock file {}", lock_path.display()))?;
        match try_lock(&file) {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => Err(KeyStoreLocked { lock_path }.into()),
            Err(TryLockError::Error(e)) if e.kind() == std::io::ErrorKind::Unsupported => {
                Err(anyhow::Error::new(e).context(format!(
                    "the file system cannot lock {}, and a key store that is not locked \
                     can lose a key to a second handle, so it is not opened",
                    lock_path.display()
                )))
            }
            Err(TryLockError::Error(e)) => Err(anyhow::Error::new(e)
                .context(format!("lock the key store file {}", lock_path.display()))),
        }
    }
}

/// `<store file>.lock`, in the store's directory, with the directory resolved.
///
/// Resolved so that two spellings of one file, a relative path and an absolute one, or
/// one through a symlinked directory or a `..`, name one lock file: otherwise each
/// would lock its own and nothing would be locked. The directory has to exist, as it
/// has to for the store's first save, which writes a temp file into it.
fn lock_path(store_path: &Path) -> Result<PathBuf> {
    let dir = store_path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let dir = dir
        .canonicalize()
        .with_context(|| format!("resolve the key store's directory {}", dir.display()))?;
    let name = store_path
        .file_name()
        .with_context(|| format!("{} names no file", store_path.display()))?;
    let mut lock_name = name.to_owned();
    lock_name.push(".lock");
    Ok(dir.join(lock_name))
}
