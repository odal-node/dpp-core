//! Helpers the register's test modules share.

/// Whether `text` is a `YYYY-MM-DD` date with a month and day in range.
///
/// Calendar validity (31 February) is not checked: the point is to refuse a
/// date that is prose, empty or in another format, and the records' dates all
/// sort as strings only because they have this shape.
pub(super) fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        && (1..=12).contains(&text[5..7].parse::<u8>().unwrap_or(0))
        && (1..=31).contains(&text[8..10].parse::<u8>().unwrap_or(0))
}
