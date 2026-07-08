/// Whether DID documents come from the dossier's own embedded snapshot, or
/// were re-fetched live. `Online` re-fetching is a native-CLI-only concern
/// (this crate's core stays offline-only); see the `odal verify` command in
/// `dpp-engine`'s CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerifyMode {
    #[default]
    Embedded,
    Online,
}

/// Outcome of a single named check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail(String),
    /// Not a failure — the layer is legitimately absent in v1 (checkpoint,
    /// calc receipts) or not applicable to this passport (no transfer chain).
    Absent(String),
}

impl CheckStatus {
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, CheckStatus::Fail(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub name: &'static str,
    pub status: CheckStatus,
}

impl CheckResult {
    #[must_use]
    fn is_failure(&self) -> bool {
        self.status.is_failure()
    }
}

/// The full result of verifying a dossier.
#[derive(Debug, Clone)]
pub struct VerificationReport {
    pub mode: VerifyMode,
    pub trust_anchor_note: String,
    pub checks: Vec<CheckResult>,
}

impl VerificationReport {
    /// `true` iff every check passed (informational `Absent` checks don't
    /// count against this).
    #[must_use]
    pub fn all_verified(&self) -> bool {
        !self.checks.iter().any(CheckResult::is_failure)
    }

    /// Exit-code convention: `0` verified, `1` any tamper, `2` never
    /// returned from here — malformed/incomplete input (including an
    /// unrecognised field) is a hard parse error before a report can even be
    /// built; see `verify_dossier_json`.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        if self.all_verified() { 0 } else { 1 }
    }
}
