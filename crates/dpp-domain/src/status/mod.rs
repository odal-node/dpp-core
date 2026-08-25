//! [`PassportStatus`] — the passport lifecycle state machine.

#[cfg(test)]
mod all_tests;
mod lifecycle;
#[cfg(test)]
mod tests;

pub use lifecycle::PassportStatus;
