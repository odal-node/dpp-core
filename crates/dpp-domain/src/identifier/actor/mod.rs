//! The EN 18219 clause 6 identifier of an economic operator or a facility, typed
//! by the scheme it follows.
//!
//! Where an operator or a facility is named by a free `scheme` string and a
//! value, this says which clause 6 scheme the pair is: scheme 6, a scheme
//! registered under ISO/IEC 6523 and told apart by its ICD, or scheme 7 in its
//! GS1 form, a GLN qualified by the Application Identifier that says what it
//! names. [`ActorIdentifier::from_legacy`] is the one place a legacy scheme string
//! is read, and what it cannot place it says so about instead of guessing.

mod error;
mod gln_ai;
mod icd;
mod identifier;
mod position;
#[cfg(test)]
mod tests;

pub use error::ActorIdentifierError;
pub use gln_ai::GlnAi;
pub use icd::Icd;
pub use identifier::ActorIdentifier;
pub use position::ActorPosition;
