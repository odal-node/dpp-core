//! Why a JWS protected header may not be acted on.

/// The reason [`header_refusal`](super::verifier::header_refusal) refuses a
/// protected header, before any signature is looked at.
///
/// Each variant is a different thing for a caller to say: a header nobody could
/// read, one that marks an extension the recipient must understand (RFC 7515
/// §4.1.11), and one whose `alg` is not the algorithm of the key it is checked
/// against.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeaderRefusal {
    /// The first segment is not base64url, or does not decode to a JSON object.
    Unreadable,
    /// The header carries `crit`, of any shape. This crate understands no
    /// critical extension, so none is accepted (RFC 7515 §4.1.11).
    Critical,
    /// `alg` is not the algorithm of the key the token is checked against: it
    /// names another one, it is `none`, or it is absent or not a string.
    Algorithm {
        /// What the header says, when it is a string; `None` when `alg` is
        /// absent or not a string.
        found: Option<String>,
    },
}
