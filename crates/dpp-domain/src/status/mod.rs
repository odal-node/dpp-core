//! [`PassportStatus`] — the passport lifecycle state machine.

mod lifecycle;
#[cfg(test)]
mod tests;

pub use lifecycle::PassportStatus;
