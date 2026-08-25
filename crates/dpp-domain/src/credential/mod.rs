//! W3C Verifiable Credential 2.0 types binding a passport to its signed payload.

mod passport;
mod signed;
mod subject;

pub use passport::PassportCredential;
pub use signed::SignedCredential;
pub use subject::PassportCredentialSubject;
