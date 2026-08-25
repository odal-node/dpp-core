//! eIDAS qualified electronic seal value objects — what a seal *is* once
//! produced, and what an adapter reports about its own capabilities.
//!
//! These are **persisted domain values**, not ports: [`SealedEnvelope`] is
//! serialised onto `Passport::seal` and travels on the wire. The trait that
//! produces one is the extension seam and lives in [`crate::ports::seal`],
//! which also carries the regulatory basis for what "qualified" requires.

mod capabilities;
mod conformance_level;
mod credential_ref;
mod envelope;
mod format;
mod indication;
mod mode;
mod request;
mod sealed_envelope;
#[cfg(test)]
mod tests;
mod verification;

pub use capabilities::SealCapabilities;
pub use conformance_level::SealConformanceLevel;
pub use credential_ref::SealCredentialRef;
pub use envelope::SealEnvelope;
pub use format::SealFormat;
pub use indication::SealIndication;
pub use mode::SealMode;
pub use request::SealRequest;
pub use sealed_envelope::SealedEnvelope;
pub use verification::{SealChecks, SealVerification};
