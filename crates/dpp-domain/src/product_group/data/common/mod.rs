//! Payload types shared by more than one product group.
//!
//! Each type here is used by at least two sibling product groups — that is the
//! condition for being in this directory at all (CODE-LAYOUT.md rule 15). A type
//! only one group uses belongs in that group.

mod critical_raw_material;
mod production_route;
mod svhc;

pub use critical_raw_material::CriticalRawMaterial;
pub use production_route::ProductionRoute;
pub use svhc::SvhcSubstance;
