//! Plugin call outcomes: [`PluginResult`] (typed compliance determination)
//! and [`AbiResult`] (the wire envelope carrying it or a [`PluginError`]).

mod plugin_result;

pub use plugin_result::*;
