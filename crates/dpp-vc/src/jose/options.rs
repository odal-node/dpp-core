//! What a caller decides before it verifies a token: the time, its clock-skew allowance,
//! the audience it identifies as, and, for an SD-JWT, whether Key Binding is required.

use std::time::Duration;

use dpp_crypto::sd_jwt::KeyBindingPolicy;

/// The greatest clock-skew allowance a verifier can configure, in seconds.
pub const MAX_LEEWAY_SECONDS: u64 = 300;

/// The time, clock-skew allowance, and audience a verifier checks a token against.
///
/// The time and audience are the caller's to supply; the leeway defaults to zero:
///
/// - **`now`** is seconds since the Unix epoch, supplied rather than read, as in
///   [`sd_jwt_vc::verify`](crate::sd_jwt_vc::verify): a result that depends on a hidden
///   clock cannot be reproduced. With a `now`, `exp` and `nbf` are checked (RFC 7519
///   §4.1.4 and §4.1.5). Without one they are not read.
/// - **`leeway`** is an optional allowance for clock skew, applied alike to `exp` and `nbf`.
///   It defaults to zero and is capped at [`MAX_LEEWAY_SECONDS`].
/// - **`audience`** is the value the caller identifies itself with. RFC 7519 §4.1.3 says
///   "If the principal processing the claim does not identify itself with a value in the
///   "aud" claim when this claim is present, then the JWT MUST be rejected." So a token
///   whose `aud` does not name it is refused, and a caller that gives no audience
///   identifies itself with nothing, so a token that carries `aud` is refused. A token
///   with no `aud` is not affected by either case.
///
/// `VerifyOptions::new()` is `now` unset, no leeway, and no audience, which checks no time
/// and refuses a token that carries `aud`.
///
/// ```
/// use dpp_vc::jose::VerifyOptions;
///
/// let options = VerifyOptions::new()
///     .at(1_800_000_000)
///     .with_leeway(30)
///     .for_audience("https://verifier.example");
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VerifyOptions<'a> {
    pub(crate) now: Option<i64>,
    pub(crate) leeway: Duration,
    pub(crate) audience: Option<&'a str>,
}

impl<'a> VerifyOptions<'a> {
    /// No time, leeway, or audience.
    pub const fn new() -> Self {
        Self {
            now: None,
            leeway: Duration::ZERO,
            audience: None,
        }
    }

    /// Check `exp` and `nbf` against `now`, in seconds since the Unix epoch.
    #[must_use]
    pub const fn at(mut self, now: i64) -> Self {
        self.now = Some(now);
        self
    }

    /// Allow up to `seconds` of clock skew for `exp` and `nbf` checks.
    ///
    /// Values above [`MAX_LEEWAY_SECONDS`] are capped at that limit. The default is zero.
    #[must_use]
    pub const fn with_leeway(mut self, seconds: u64) -> Self {
        let seconds = if seconds > MAX_LEEWAY_SECONDS {
            MAX_LEEWAY_SECONDS
        } else {
            seconds
        };
        self.leeway = Duration::from_secs(seconds);
        self
    }

    /// Identify as `audience`: a token whose `aud` names it is accepted.
    #[must_use]
    pub const fn for_audience(mut self, audience: &'a str) -> Self {
        self.audience = Some(audience);
        self
    }

    pub(crate) const fn leeway(self) -> Duration {
        self.leeway
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
