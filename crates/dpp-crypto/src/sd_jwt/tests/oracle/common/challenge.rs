//! The Verifier every bound case is for: its audience, its challenge and its window.

/// The Verifier every case is for.
pub(crate) const AUDIENCE: &str = "https://verifier.example";

/// The challenge every case answers.
pub(crate) const NONCE: &str = "n-2RsH_0kQ8vLz";

/// How far `iat` may lie from now either way, in seconds. One number for both
/// directions, because one of the other implementations takes its window as a
/// point and a symmetric skew.
pub(crate) const WINDOW: u32 = 300;
