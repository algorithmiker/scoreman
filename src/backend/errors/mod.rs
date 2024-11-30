use std::{
    cmp::{max, min},
    ops::RangeInclusive,
};

pub mod backend_error;
pub mod backend_error_kind;
pub mod diagnostic;
pub mod diagnostic_kind;
pub mod error_location;

pub const ERROR_CONTEXT: usize = 3;

/// When reporting an error with `relevant_lines`, we want to show some context of +=, [ERROR_CONTEXT]
/// lines that are not neccessarily relevant
/// This is not always possible because of line bounds (cannot show 3 lines before line 0)
/// This function handles that.
pub fn extend_error_range(range: &RangeInclusive<usize>, line_cnt: usize) -> RangeInclusive<usize> {
    let start = max(*range.start(), ERROR_CONTEXT) - ERROR_CONTEXT;
    let end = min(line_cnt - 1, range.end() + ERROR_CONTEXT);

    start..=end
}
