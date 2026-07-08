use super::entry::{AuditEntry, GENESIS_PREV_HASH};

/// The first broken link found while verifying a passport's audit chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditChainBreak {
    /// 0-based position of the offending entry in the ordered chain.
    pub index: usize,
    /// The passport whose chain broke.
    pub passport_id: String,
    /// Human-readable reason (prev-link mismatch vs. content tamper).
    pub reason: String,
}

/// Verify a passport's audit entries form an intact hash chain.
///
/// `entries` must be in chain order (ascending timestamp). Returns the first
/// break: either a `prev_hash` that doesn't point at the prior entry, or an
/// `entry_hash` that doesn't match the entry's recomputed content hash (a
/// tampered row). `Ok(())` means every link verifies.
///
/// This detects any tamper that does not re-hash the *entire forward chain*;
/// pinning the head with a signed checkpoint is what makes a full re-hash
/// detectable by a third party without access to the issuing node.
///
/// # Errors
/// [`AuditChainBreak`] at the first inconsistent entry.
pub fn verify_audit_chain(entries: &[AuditEntry]) -> Result<(), AuditChainBreak> {
    let mut expected_prev = GENESIS_PREV_HASH.to_owned();
    for (index, entry) in entries.iter().enumerate() {
        let stored_prev = entry.prev_hash.as_deref().unwrap_or(GENESIS_PREV_HASH);
        if stored_prev != expected_prev {
            return Err(AuditChainBreak {
                index,
                passport_id: entry.passport_id.clone(),
                reason: format!(
                    "prev_hash link broken: stored {stored_prev:?}, expected {expected_prev:?}"
                ),
            });
        }
        let recomputed = entry.chain_hash(&expected_prev);
        let stored_hash = entry.entry_hash.as_deref().unwrap_or("");
        if stored_hash != recomputed {
            return Err(AuditChainBreak {
                index,
                passport_id: entry.passport_id.clone(),
                reason: "entry_hash mismatch — content tampered".to_owned(),
            });
        }
        expected_prev = recomputed;
    }
    Ok(())
}
