use crate::backend::errors::backend_error_kind::BackendErrorKind;
use crate::backend::errors::diagnostic::Diagnostic;
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
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> BackendError<'a> {
    pub fn from_io_error(x: std::io::Error, diagnostics: Vec<Diagnostic>) -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            kind: BackendErrorKind::IOError(x),
            relevant_lines: 0..=0,
            diagnostics,
        }
    }
    pub fn from_fmt_error(x: std::fmt::Error, diagnostics: Vec<Diagnostic>) -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            //short: "Cannot write to internal buffer".to_string(),
            //long: format!("Format error:\n{x}"),
            diagnostics,
            relevant_lines: 0..=0,
            kind: BackendErrorKind::FmtError(x),
        }
    }

    pub fn empty_score_err(diagnostics: Vec<Diagnostic>) -> Self {
        BackendError {
            main_location: ErrorLocation::NoLocation,
            diagnostics,
            kind: BackendErrorKind::EmptyScore,
            relevant_lines: 0..=0,
        }
    }

    pub fn no_such_fret(
        location_a: usize,
        location_b: usize,
        string_name: char,
        fret: u8,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            main_location: ErrorLocation::LineAndMeasure(location_a, location_b),
            diagnostics,
            kind: BackendErrorKind::NoSuchFret(string_name, fret),
            relevant_lines: location_a..=location_a,
        }
    }
    pub fn bad_multichar_tick(
        diagnostics: Vec<Diagnostic>,
        parent_line: usize,
        chr: usize,
        multichar_track: char,
        multichar_fret: u8,
        invalid_track: char,
        invalid_fret: u8,
        tick_idx: usize,
    ) -> Self {
        Self {
            main_location: ErrorLocation::LineAndCharIdx(parent_line, chr),
            relevant_lines: parent_line..=parent_line,
            kind: BackendErrorKind::BadMulticharTick {
                multichar: (multichar_track, multichar_fret),
                invalid: (invalid_track, invalid_fret),
                tick_idx,
            },
            diagnostics,
        }
    }
}
