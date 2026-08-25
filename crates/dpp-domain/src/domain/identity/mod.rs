//! Identity and access types: `Audience`, `Disclosure`, `SignedCredential`, and `PassportCredential`.

mod audience;
mod credential_subject;
mod disclosure;
mod passport_credential;
mod signed_credential;
#[cfg(test)]
mod tests;

pub use audience::Audience;
pub use credential_subject::PassportCredentialSubject;
pub use disclosure::{Disclosure, PASSPORT_FIELD_DISCLOSURE, disclosure_key};
pub use passport_credential::PassportCredential;
pub use signed_credential::SignedCredential;
