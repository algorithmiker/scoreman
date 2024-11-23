use crate::backend::errors::backend_error_kind::BackendErrorKind;
use crate::backend::errors::error_location::ErrorLocation;
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
pub struct BackendError<'a> {
    pub main_location: ErrorLocation,
    pub relevant_lines: RangeInclusive<usize>,
    pub kind: BackendErrorKind<'a>,
}

impl<'a> BackendError<'a> {
    pub fn empty_score_err() -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            kind: BackendErrorKind::EmptyScore,
            relevant_lines: 0..=0,
        }
    }

    pub fn no_such_fret(location_a: usize, location_b: usize, string_name: char, fret: u8) -> Self {
        Self {
            main_location: ErrorLocation::LineAndMeasure(location_a, location_b),
            kind: BackendErrorKind::NoSuchFret(string_name, fret),
            relevant_lines: location_a..=location_a,
        }
    }
}

impl<'a> From<std::io::Error> for BackendError<'a> {
    fn from(value: std::io::Error) -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            kind: BackendErrorKind::IOError(value),
            relevant_lines: 0..=0,
        }
    }
}
impl<'a> From<std::fmt::Error> for BackendError<'a> {
    fn from(value: std::fmt::Error) -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            relevant_lines: 0..=0,
            kind: BackendErrorKind::FmtError(value),
        }
    }
}
