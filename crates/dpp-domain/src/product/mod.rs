//! [`ProductIdentity`] — what identifies a product, independent of its passport.

mod identity;
#[cfg(test)]
mod tests;

pub use identity::ProductIdentity;
