//! [`ManufacturerInfo`] embedded in the passport.

use serde::{Deserialize, Serialize};

/// Manufacturer information embedded in the passport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManufacturerInfo {
    pub name: String,
    pub address: String,
    /// The manufacturer's `did:web` URL, e.g. `https://acme.example.com/.well-known/did.json`
    pub did_web_url: Option<String>,
}
