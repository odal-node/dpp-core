use anyhow::Result;

use super::entry::{KeyEntry, SigningMaterial};
use super::store::{KeyRecord, KeyRecordMap, KeyStore};
use crate::jws::algorithm::KeyAlgorithm;

/// How many times a rotation that keeps the algorithm picks it, generates the new
/// key and tries the swap before it gives up. A second attempt succeeds unless
/// another rotation changed the algorithm in between.
const MAX_ROTATION_ATTEMPTS: usize = 3;

/// Build the archived-record key for `key_id`.
///
/// Suffixed with a time-ordered UUID (v7) rather than a raw nanosecond
/// timestamp: two rotations landing on the same clock tick (or a tight
/// successive-rotation loop, e.g. in tests) previously collided on the same
/// map key, silently overwriting an already-archived record — a signature
/// made with the overwritten key would then never verify again. `load_archived_keys`'s
/// `entries.sort_by_key` still recovers chronological order, since a v7 UUID's
/// leading bits are a millisecond timestamp.
pub(super) fn archived_key_name(key_id: &str) -> String {
    format!("{key_id}{ARCHIVE_MARKER}{}", uuid::Uuid::now_v7())
}

/// What an archived record's key carries between the key's id and its time-ordered suffix.
/// A key id that contains it is an archive, and one that starts with `{key_id}` and this is
/// an archive of `key_id`.
pub(super) const ARCHIVE_MARKER: &str = "#archived-";

/// Refuse a key id that could be read as an archive.
///
/// An archived record is told from a current one by [`ARCHIVE_MARKER`] in its key, so a
/// current key named with it would be left out of `list` and returned as a predecessor of
/// the key whose id it begins with. Every path that records a current key checks this first.
///
/// # Errors
/// When `key_id` contains [`ARCHIVE_MARKER`].
pub(super) fn check_key_id(key_id: &str) -> Result<()> {
    anyhow::ensure!(
        !key_id.contains(ARCHIVE_MARKER),
        "key id {key_id:?} cannot contain `{ARCHIVE_MARKER}`: a rotation names an archived \
         key that way, and a current key with that name would be read as an archive"
    );
    Ok(())
}

/// The algorithm a rotation of `key_id` keeps: the current record's, or the
/// default when the key has no record yet.
fn algorithm_of(map: &KeyRecordMap, key_id: &str) -> KeyAlgorithm {
    map.get(key_id)
        .map_or_else(super::store::default_algorithm, |r| r.algorithm)
}

impl KeyStore {
    /// Archive the current key under a timestamped key so it can still be used
    /// to verify older signatures after rotation.
    pub fn archive_key(&self, key_id: &str) -> Result<()> {
        if *self.needs_migration.read().expect("lock") {
            anyhow::bail!(
                "key store requires KDF migration before writes are allowed — \
                 open it with `KeyStore::open_and_migrate`"
            );
        }
        let mut map = self.records.write().expect("key store write lock poisoned");
        if let Some(record) = map.get(key_id).cloned() {
            let mut proposed = map.clone();
            proposed.insert(archived_key_name(key_id), record);
            self.persist_envelope(&proposed)?;
            *map = proposed;
        }
        Ok(())
    }

    /// Atomically rotate the key: archive the current key (kept valid so older
    /// signatures still verify) and install a fresh current key, in a **single**
    /// persisted write. Use for routine/hygiene rotation. Returns the new key.
    ///
    /// A current key that is already revoked is archived revoked: a rotation never
    /// makes a revoked key valid again.
    ///
    /// Unlike calling [`archive_key`](Self::archive_key) then
    /// [`generate_key`](Self::generate_key), there is no intermediate on-disk
    /// state where the archive exists but the new key does not (identity I4).
    pub fn rotate_key(&self, key_id: &str) -> Result<KeyEntry> {
        self.rotate_inner(key_id, false, None)
    }

    /// [`rotate_key`](Self::rotate_key), installing a key of `algorithm`.
    ///
    /// The archived key keeps its own algorithm, so a DID document built after
    /// a cross-algorithm rotation lists both, each under its own JWK shape, and
    /// a signature made before the rotation still resolves to the key that made
    /// it.
    pub fn rotate_key_to(&self, key_id: &str, algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        self.rotate_inner(key_id, false, Some(algorithm))
    }

    /// Atomically **revoke** the current key and install a fresh one. The old key
    /// is archived but marked revoked, so `dpp_vc::did_builder` drops it from
    /// the published DID document and signatures it produced no longer verify.
    /// Use this on key **compromise** (vs. hygiene rotation, where the old key
    /// stays valid — see [`rotate_key`](Self::rotate_key)).
    pub fn revoke_and_rotate(&self, key_id: &str) -> Result<KeyEntry> {
        self.rotate_inner(key_id, true, None)
    }

    pub(super) fn rotate_inner(
        &self,
        key_id: &str,
        revoke_old: bool,
        algorithm: Option<KeyAlgorithm>,
    ) -> Result<KeyEntry> {
        check_key_id(key_id)?;
        if *self.needs_migration.read().expect("lock") {
            anyhow::bail!(
                "key store requires KDF migration before writes are allowed — \
                 open it with `KeyStore::open_and_migrate`"
            );
        }

        // Each attempt picks the algorithm, then generates and seals the new key
        // with no lock held, so an RNG failure cannot poison the store, and only
        // then takes the write lock. A rotation that keeps the algorithm checks,
        // under that lock, that the algorithm it picked is still the current one,
        // and tries again when another rotation has changed it. The new state is
        // built on a copy of the map and installed only after the copy is saved, so
        // a failed write leaves the handle as the file has it.
        //
        // The new key is sealed to its own fingerprint. The archived copy keeps the
        // fingerprint it was sealed under, which is why the binding is to the
        // fingerprint rather than the map key it is filed at.
        for _ in 0..MAX_ROTATION_ATTEMPTS {
            let chosen = match algorithm {
                Some(explicit) => explicit,
                None => {
                    let map = self
                        .records
                        .read()
                        .map_err(|_| anyhow::anyhow!("key store lock poisoned"))?;
                    algorithm_of(&map, key_id)
                }
            };
            let (new_record, entry) = self.seal_new_record(SigningMaterial::generate(chosen))?;

            let mut map = self.records.write().expect("key store write lock poisoned");
            if algorithm.is_none() && algorithm_of(&map, key_id) != chosen {
                continue;
            }
            let mut proposed = map.clone();
            // Archive the existing current key (if any), marking it revoked when
            // this is a compromise rotation.
            if let Some(record) = map.get(key_id).cloned() {
                if revoke_old {
                    // Earlier archives made by `archive_key` hold this same key
                    // material. A compromise must revoke every copy in the same
                    // write, or the compromised key stays in the published DID
                    // document and its signatures keep verifying.
                    for stored in proposed.values_mut() {
                        if stored.fingerprint == record.fingerprint {
                            stored.revoked = true;
                        }
                    }
                }
                // A key revoked already stays revoked: a hygiene rotation of a
                // revoked current key must not archive it as a valid one, or it
                // would be published again and its signatures would verify.
                let archived = KeyRecord {
                    revoked: revoke_old || record.revoked,
                    ..record
                };
                proposed.insert(archived_key_name(key_id), archived);
            }
            proposed.insert(key_id.to_owned(), new_record);
            self.persist_envelope(&proposed)?;
            *map = proposed;
            return Ok(entry);
        }
        anyhow::bail!("concurrent rotations kept changing the algorithm of `{key_id}`; try again")
    }
}
