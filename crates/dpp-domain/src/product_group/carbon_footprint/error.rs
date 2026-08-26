//! [`CarbonFootprintClassError`] — why a performance-class label was refused.

/// Error from constructing a [`CarbonFootprintClass`](super::CarbonFootprintClass).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CarbonFootprintClassError {
    #[error("carbon footprint class label must not be empty or blank")]
    Empty,
    #[error(
        "carbon footprint class label '{label}' is {len} characters, \
         exceeding the maximum of {max}"
    )]
    TooLong {
        label: String,
        len: usize,
        max: usize,
    },
    #[error("carbon footprint class label '{0}' contains a control character")]
    ControlCharacter(String),
}
