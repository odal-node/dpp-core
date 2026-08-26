//! [`CnCategory`] — a CN chapter (2 digits) or heading (4).

mod category;
mod error;
#[cfg(test)]
mod tests;

pub use category::CnCategory;
pub use error::CnCategoryError;
