//! The lock that gives a key store file one writer.
//!
//! It is taken on a **sibling file**, `<store file>.lock`, and not on the store
//! file: every write replaces the store by renaming a temp file over it, so a lock on
//! the old file would guard nothing once the first write had happened. The lock file
//! is never renamed or removed, so every handle that opens the store locks the same
//! file.
//!
//! Two handles lock the same file only if they name the store file the same way, so
//! the name is settled once, here, by [`resolve`], and the store reads and saves
//! through the path it returns and not through the one it was given. A store file
//! cannot be named `*.lock`: that is the name of a lock file, and a store saved over
//! another store's lock file would leave that lock on a file nothing opens.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::error::KeyStoreLocked;

/// An exclusive lock on a store's lock file, held until it is dropped.
///
/// Dropping it closes the file, which releases the lock; so does the process ending,
/// however it ends.
pub(crate) struct StoreLock {
    /// The store file the lock is for, as [`resolve`] settled it. The store reads and
    /// saves through this path, so what is locked is what is written.
    store_path: PathBuf,
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
    /// [`KeyStoreLocked`] when another handle holds the lock. Otherwise the path
    /// cannot be resolved or names a lock file, the lock file could not be made or
    /// opened, or the file system cannot lock files: an unlocked store is the defect
    /// this lock exists to prevent, so that refuses the open too.
    pub(crate) fn acquire(store_path: &Path) -> Result<Self> {
        Self::acquire_with(store_path, File::try_lock)
    }

    /// [`Self::acquire`], with the locking call supplied, so a test can say what a
    /// platform that cannot lock answers.
    pub(crate) fn acquire_with(
        store_path: &Path,
        try_lock: impl FnOnce(&File) -> Result<(), TryLockError>,
    ) -> Result<Self> {
        let store_path = resolve(store_path)?;
        let mut lock_name = store_path.clone().into_os_string();
        lock_name.push(".lock");
        let lock_path = PathBuf::from(lock_name);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("open the key store lock file {}", lock_path.display()))?;
        match try_lock(&file) {
            Ok(()) => Ok(Self {
                store_path,
                _file: file,
            }),
            Err(TryLockError::WouldBlock) => Err(KeyStoreLocked { lock_path }.into()),
            Err(TryLockError::Error(e)) if e.kind() == ErrorKind::Unsupported => {
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

    /// The store file this lock is for, resolved. Read and save through this path.
    pub(crate) fn store_path(&self) -> &Path {
        &self.store_path
    }
}

/// The one path a store file goes by.
///
/// Two spellings of one file name one lock file only if they resolve to one path, so
/// every way of reaching a store file is settled here: a relative path and an absolute
/// one, a `..`, a symlinked directory, and a store file that is itself a symlink, which
/// resolves to its target. Without that, each spelling would lock its own file and
/// nothing would be locked. A store file that is not there yet is resolved by its
/// directory, which has to exist, as it has to for the first save, which writes a temp
/// file into it.
///
/// The store then reads and saves through the resolved path. A save renames a temp file
/// over the store file, which replaces a symlink and not the file it points at, so a
/// store opened through a link would otherwise stop being the file the link names.
///
/// A path that names a lock file, `*.lock`, is refused: a store saved there would
/// replace the lock file of the store named without the suffix, and leave that lock on
/// a file nothing opens.
fn resolve(store_path: &Path) -> Result<PathBuf> {
    let resolved = match store_path.canonicalize() {
        Ok(resolved) => resolved,
        Err(e) if e.kind() == ErrorKind::NotFound => {
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
            dir.join(name)
        }
        Err(e) => {
            return Err(anyhow::Error::new(e).context(format!(
                "resolve the key store file {}",
                store_path.display()
            )));
        }
    };
    let names_a_lock_file = resolved
        .file_name()
        .is_some_and(|name| name.to_string_lossy().ends_with(".lock"));
    anyhow::ensure!(
        !names_a_lock_file,
        "{} cannot be a key store file: a name ending in `.lock` is a lock file, and each \
         store keeps one beside it",
        resolved.display()
    );
    Ok(resolved)
}
