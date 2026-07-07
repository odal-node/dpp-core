//! Pure glue (host-testable, no linear-memory side effects): JSON
//! encode/decode between a [`DppSectorPlugin`] and the ABI byte buffers.

use dpp_plugin_traits::{AbiResult, DppSectorPlugin, PluginError, PluginInput};
use serde::Serialize;

pub(crate) fn to_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

fn parse_input(bytes: &[u8]) -> Result<PluginInput, PluginError> {
    serde_json::from_slice(bytes).map_err(|e| PluginError::InvalidInput(e.to_string()))
}

/// Serialise the plugin's [`PluginMeta`](dpp_plugin_traits::PluginMeta) to JSON bytes.
pub fn metadata_bytes<P: DppSectorPlugin>(plugin: &P) -> Vec<u8> {
    to_bytes(&plugin.meta())
}

/// Serialise the plugin's [`PluginCapabilities`](dpp_plugin_traits::PluginCapabilities) to JSON bytes.
pub fn describe_bytes<P: DppSectorPlugin>(plugin: &P) -> Vec<u8> {
    to_bytes(&plugin.capabilities())
}

/// Run `validate_input` and serialise the [`AbiResult`] envelope.
pub fn validate_bytes<P: DppSectorPlugin>(plugin: &P, input: &[u8]) -> Vec<u8> {
    let outcome = match parse_input(input) {
        Ok(value) => match plugin.validate_input(&value) {
            Ok(()) => AbiResult::Ok(serde_json::Value::Null),
            Err(e) => AbiResult::Error(e),
        },
        Err(e) => AbiResult::Error(e),
    };
    to_bytes(&outcome)
}

/// Run `calculate_metrics` and serialise the [`AbiResult`] envelope.
pub fn calculate_metrics_bytes<P: DppSectorPlugin>(plugin: &P, input: &[u8]) -> Vec<u8> {
    let outcome = match parse_input(input) {
        Ok(value) => match plugin.calculate_metrics(&value) {
            Ok(result) => AbiResult::ok(&result),
            Err(e) => AbiResult::Error(e),
        },
        Err(e) => AbiResult::Error(e),
    };
    to_bytes(&outcome)
}

/// Run `generate_passport` and serialise the [`AbiResult`] envelope.
pub fn generate_passport_bytes<P: DppSectorPlugin>(plugin: &P, input: &[u8]) -> Vec<u8> {
    let outcome = match parse_input(input) {
        Ok(value) => match plugin.generate_passport(&value) {
            Ok(payload) => AbiResult::Ok(payload),
            Err(e) => AbiResult::Error(e),
        },
        Err(e) => AbiResult::Error(e),
    };
    to_bytes(&outcome)
}
