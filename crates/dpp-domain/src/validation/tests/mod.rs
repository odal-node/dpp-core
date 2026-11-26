//! Tests of the `validation` module.

#[cfg(not(target_arch = "wasm32"))]
mod personal_data;
#[cfg(not(target_arch = "wasm32"))]
mod validator;
