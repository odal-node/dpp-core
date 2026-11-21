//! What a caller decides before it verifies a token: the time, the audience it identifies
//! as, and, for an SD-JWT, whether Key Binding is required.

use dpp_crypto::sd_jwt::KeyBindingPolicy;

/// The time and the audience a verifier checks a token against.
///
/// Both are the caller's to supply, and neither has a value that is assumed:
///
/// - **`now`** is seconds since the Unix epoch, supplied rather than read, as in
///   [`sd_jwt_vc::verify`](crate::sd_jwt_vc::verify): a result that depends on a hidden
///   clock cannot be reproduced. With a `now`, `exp` and `nbf` are checked (RFC 7519
///   §4.1.4 and §4.1.5). Without one they are not read.
/// - **`audience`** is the value the caller identifies itself with. RFC 7519 §4.1.3 says
///   "If the principal processing the claim does not identify itself with a value in the
///   "aud" claim when this claim is present, then the JWT MUST be rejected." So a token
///   whose `aud` does not name it is refused, and a caller that gives no audience
///   identifies itself with nothing, so a token that carries `aud` is refused. A token
///   with no `aud` is not affected by either case.
///
/// `VerifyOptions::new()` is `now` unset and no audience, which checks no time and refuses
/// a token that carries `aud`.
///
/// ```
/// use dpp_vc::jose::VerifyOptions;
///
/// let options = VerifyOptions::new().at(1_800_000_000).for_audience("https://verifier.example");
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VerifyOptions<'a> {
    pub(crate) now: Option<i64>,
    pub(crate) audience: Option<&'a str>,
}

impl<'a> VerifyOptions<'a> {
    /// No time and no audience.
    pub const fn new() -> Self {
        Self {
            now: None,
            audience: None,
        }
    }

    /// Check `exp` and `nbf` against `now`, in seconds since the Unix epoch.
    #[must_use]
    pub const fn at(mut self, now: i64) -> Self {
        self.now = Some(now);
        self
    }

    /// Identify as `audience`: a token whose `aud` names it is accepted.
    #[must_use]
    pub const fn for_audience(mut self, audience: &'a str) -> Self {
        self.audience = Some(audience);
        self
    }
}

/// Whether Key Binding is required of an SD-JWT, which RFC 9901 §9.5 says "Verifiers MUST
/// decide ... for a particular use case before verifying a credential."
///
/// There is no default, so a caller cannot verify an SD-JWT without having chosen. The
/// choice is the use case's and never the token's: §7.3 step 1 says it "MUST NOT be based
/// on whether or not a Key Binding JWT is provided by the Holder", and §9.5 explains that
/// otherwise "an attacker could strip the KB-JWT from an SD-JWT+KB and present the
/// resultant SD-JWT".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyBindingRequirement {
    /// Key Binding is not required. A Key Binding JWT the token carries is not read: the
    /// token is processed as an SD-JWT, as RFC 9901 §7.3 steps 3 and 4 say.
    NotRequired,
    /// Key Binding is required, and the Key Binding JWT must satisfy this policy: the
    /// audience and the nonce it names, and the window its `iat` falls in (§7.3 step 5).
    /// The Holder's key is the one in the `cnf` of the processed payload.
    ///
    /// The policy's `now` is the time of the Key Binding JWT's checks, and is separate from
    /// [`VerifyOptions`]' `now`, which governs the credential's own `exp` and `nbf`. The
    /// policy's audience is the one the Key Binding JWT's `aud` must name, which a caller
    /// usually also gives to [`VerifyOptions::for_audience`].
    Required(KeyBindingPolicy),
}
