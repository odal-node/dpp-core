//! Fixtures and helpers shared by the harness tests, vendored once in `dpp-vc`.

use crate::run::Verdict;

// Named from the crate root, since the modules under `tests/` invoke it from their own
// directory.
macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../dpp-vc/src/jose/fixtures/w3c-vc-jose-cose/",
            $name
        ))
    };
}

pub(super) const P256: &str = fixture!("vm-p256.json");
pub(super) const P384: &str = fixture!("vm-p384.json");
pub(super) const ED25519: &str = fixture!("vm-ed25519.json");
pub(super) const DID_ED25519: &str = fixture!("vm-did-ed25519.json");

pub(super) fn is_success(verdict: &Verdict) -> bool {
    matches!(verdict, Verdict::Success(_))
}
