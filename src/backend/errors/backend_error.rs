use crate::backend::errors::backend_error_kind::BackendErrorKind;
use crate::backend::errors::error_location::ErrorLocation;
use std::cmp::{max, min};
use std::ops::RangeInclusive;

/// Produced [diagnostics.len()] diagnostics and one error.
/// Diagnostics:
///  - [location]
///    [severity] [message]
///
/// [short]
/// [location]
/// [long]
///
#[derive(Debug)]
pub struct BackendError {
    pub main_location: ErrorLocation,
    pub relevant_lines: RangeInclusive<usize>,
    pub kind: BackendErrorKind,
}

impl BackendError {
    pub fn empty_score_err() -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            kind: BackendErrorKind::EmptyScore,
            relevant_lines: 0..=0,
        }
    }
    pub fn no_closing_barline(line_idx: usize) -> Self {
        BackendError {
            main_location: ErrorLocation::LineOnly(line_idx),
            kind: BackendErrorKind::NoClosingBarline,
            relevant_lines: line_idx..=line_idx,
        }
    }
    pub fn fixup_failed(location: ErrorLocation, relevant_lines: RangeInclusive<usize>) -> Self {
        BackendError {
            main_location: location,
            relevant_lines,
            kind: BackendErrorKind::FixupFailed,
        }
    }
    pub fn parse3_invalid_string_name(line: usize) -> Self {
        BackendError {
            main_location: ErrorLocation::LineOnly(line),
            relevant_lines: line..=line,
            kind: BackendErrorKind::InvalidStringName,
        }
    }
    pub fn parse3_invalid_character(line: u32, char: u32, c: char) -> Self {
        BackendError {
            main_location: ErrorLocation::LineAndChar(line, char),
            kind: BackendErrorKind::Parse3InvalidCharacter(c),
            relevant_lines: line as usize..=line as usize,
        }
    }
    pub fn bend_on_invalid(line: u32, char: u32) -> Self {
        BackendError {
            main_location: ErrorLocation::LineAndChar(line, char),
            kind: BackendErrorKind::BendOnInvalid,
            relevant_lines: line as usize..=line as usize,
        }
    }
    pub fn both_slots_multichar(main_line: u32, main_char: u32, other_line: u32) -> Self {
        let min = min(main_line, other_line) as usize;
        let max = max(main_line, other_line) as usize;
        BackendError {
            main_location: ErrorLocation::LineAndChar(main_line, main_char),
            kind: BackendErrorKind::BothSlotsMultiChar,
            relevant_lines: min..=max,
        }
    }
    pub fn multi_both_slots_filled(line: u32, char: u32) -> Self {
        Self {
            main_location: ErrorLocation::LineAndChar(line, char),
            kind: BackendErrorKind::MultiBothSlotsFilled,
            relevant_lines: line as usize..=line as usize,
        }
    }
}

impl From<std::io::Error> for BackendError {
    fn from(value: std::io::Error) -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            kind: BackendErrorKind::IOError(value),
            relevant_lines: 0..=0,
        }
    }
}
impl From<std::fmt::Error> for BackendError {
    fn from(value: std::fmt::Error) -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            relevant_lines: 0..=0,
            kind: BackendErrorKind::FmtError(value),
        }
    }
}
