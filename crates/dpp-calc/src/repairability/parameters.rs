//! Input parameters for the simplified, non-regulatory repairability heuristic
//! (see the module-level note in `repairability/mod.rs` — this is **not** the
//! enacted EU 2023/1669 Annex IV index).

use serde::{Deserialize, Serialize};

/// Six-parameter repairability inputs for one product.
///
/// Each parameter uses a three-level ordinal scale (in the style of EN 45554's
/// per-criterion scoring, but not a faithful implementation of that standard):
/// `0` = criterion not met, `1` = criterion partially met, `2` = criterion
/// fully met. Values outside `[0, 2]` are rejected at calculation time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairabilityInputs {
    /// Ease of product disassembly (fastener types, tools required, destructive entry).
    /// 0 = non-destructive entry not possible; 1 = possible with non-standard tools;
    /// 2 = easy with common tools, no irreversible steps.
    pub disassembly: u8,

    /// Availability of spare parts through OEM and independent aftermarket channels.
    /// 0 = not available; 1 = available through IAM only or limited SKUs;
    /// 2 = full OEM + IAM availability for the supported lifetime.
    pub spare_parts: u8,

    /// Availability of repair and maintenance documentation for professional repairers.
    /// 0 = none; 1 = limited/partial; 2 = full service manual + schematics publicly available.
    pub repair_info: u8,

    /// Availability of software diagnostic tools and processes for fault isolation.
    /// 0 = none; 1 = basic fault codes only; 2 = full diagnostic suite available to repairers.
    pub diagnostic_tools: u8,

    /// Software and firmware updatability for the duration of the support period.
    /// 0 = no updates provided; 1 = security patches only; 2 = full OS/firmware + long-term commitment.
    pub software_updatability: u8,

    /// Customer-related aspects: warranty terms, authorised repair network, support channels.
    /// 0 = poor (< 1 year warranty, no repair network); 1 = standard; 2 = extended warranty + wide repair network.
    pub customer_support: u8,
}
