//! [`EnergyEfficiencyClass`] — the EU energy label class.

use serde::{Deserialize, Serialize};

/// EU energy label class per EU Energy Labelling Regulation 2017/1369 (A–G scale).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EnergyEfficiencyClass {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    #[serde(other)]
    Other,
}
