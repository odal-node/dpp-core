//! The `Passport` aggregate root and its unique identifier type.

pub mod category;
pub mod facility;
pub mod id;
pub mod manufacturer;
pub mod material;
#[allow(clippy::module_inception)]
pub mod passport;
pub mod reference;
pub mod view;

#[cfg(test)]
mod tests;

pub use category::ProductCategory;
pub use facility::FacilitySnapshot;
pub use id::PassportId;
pub use manufacturer::ManufacturerInfo;
pub use material::MaterialEntry;
pub use passport::Passport;
pub use reference::PassportRef;
pub use view::PassportView;
