//! Batch validation of multiple sector-data items in one pass.

use super::functions::validate_sector_data;
use crate::domain::field_error::ValidationErrors;
use crate::domain::sector::SectorData;

/// Result of validating a single item in a batch.
#[derive(Debug, Clone)]
pub struct BatchValidationItem {
    /// Zero-based index in the input slice.
    pub index: usize,
    /// Validation result: `Ok(())` if valid, `Err` with field-level errors otherwise.
    pub result: Result<(), ValidationErrors>,
}

/// Validate a batch of sector data items, collecting all errors per item.
///
/// The returned `Vec` has the same length and order as the input.
pub fn validate_sector_data_batch(items: &[SectorData]) -> Vec<BatchValidationItem> {
    items
        .iter()
        .enumerate()
        .map(|(index, data)| BatchValidationItem {
            index,
            result: validate_sector_data(data),
        })
        .collect()
}

/// Returns only the failures from a batch validation run.
pub fn batch_errors(results: &[BatchValidationItem]) -> Vec<&BatchValidationItem> {
    results.iter().filter(|item| item.result.is_err()).collect()
}
