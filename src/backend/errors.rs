use std::{
    cmp::{max, min},
    fmt::{self, Display},
    ops::RangeInclusive,
};

use yansi::Paint;

use crate::collect_parse_error;

#[derive(Clone, Debug)]
pub enum ErrorLocation {
    NoLocation,
    LineOnly(usize),
    LineAndMeasure(usize, usize),
    LineAndCharIdx(usize, usize),
}
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

#[derive(Debug)]
pub enum BackendErrorKind<'a> {
    IOError(std::io::Error),
    FmtError(std::fmt::Error),
    EmptyScore,
    /// string name and fret
    NoSuchFret(char, u16),
    TickMismatch(char, char, usize, usize),
    /// string name where tick is multichar, string name here, tick idx, and the found invalid fret
    BadMulticharTick {
        /// string and fret
        multichar: (char, u16),
        /// string and fret
        invalid: (char, u16),
        tick_idx: usize,
    },
    InvalidCommentSyntax(String),
    InvalidPartlineSyntax(String),
    ParseError(nom::Err<nom::error::VerboseError<&'a str>>),
}

impl<'a> BackendErrorKind<'a> {
    pub fn desc(&self) -> (String, String) {
        match self {
            BackendErrorKind::IOError(x) => {
                ("Cannot write to file".into(), format!("IO Error:\n{x}"))
            }

            BackendErrorKind::FmtError(x) => {
                ("Cannot write to internal buffer".into(), format!("Format error:\n{x}"))
            }
            BackendErrorKind::EmptyScore => ("Empty score".into(), String::new()),
            BackendErrorKind::NoSuchFret(string_name, fret) => (
                "No such fret".into(),
                format!("Failed to get note for fret {fret} on string {string_name}"),
            ),

            BackendErrorKind::TickMismatch(string_before, string_after,ticks_before, ticks_after) => ("Tick mismatch".into(),
format!("The muxml2 backend relies on the fact that there are the same number of ticks (frets/rests) on every line (string) of a measure in the tab. This is not true for this tab.
The measure has {ticks_before} ticks on string {string_before} and {ticks_after} ticks on string {string_after}.

Tip: If you get a lot of errors like this, consider using the muxml1 backend.")
            ),
            BackendErrorKind::BadMulticharTick { multichar : (multichar_string,multichar_fret), invalid: (invalid_string,invalid_fret), tick_idx } =>
            (
                "Invalid multichar tick".into(), 
                format!(
"Tick {} has a multi-char fret ({multichar_fret}) on string {multichar_string}, but on the same tick there is an invalid fret {invalid_fret} on string {invalid_string}", tick_idx+1)
            ),
            BackendErrorKind::InvalidCommentSyntax(rem) => ("Invalid comment syntax".into(), format!("Got remaining content: `{rem}`")),
            BackendErrorKind::InvalidPartlineSyntax(rem) => ("Invalid partline syntax".into(), format!("Got remaining content: `{rem}`")),
            BackendErrorKind::ParseError(x) => ("Invalid syntax".into(), collect_parse_error(x)),

        }
    }
}
#[derive(Debug, Clone)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
}
impl Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DiagnosticSeverity::Info => "Info".blue(),
                DiagnosticSeverity::Warning => "Warning".yellow(),
            }
        )
    }
}
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub location: ErrorLocation,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

impl Diagnostic {
    pub fn info(location: ErrorLocation, message: String) -> Self {
        Self {
            location,
            message,
            severity: DiagnosticSeverity::Info,
        }
    }
    pub fn warn(location: ErrorLocation, message: String) -> Self {
        Self {
            location,
            message,
            severity: DiagnosticSeverity::Warning,
        }
    }
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
        fret: u16,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            main_location: ErrorLocation::LineAndMeasure(location_a, location_b),
            diagnostics,
            kind: BackendErrorKind::NoSuchFret(string_name, fret),
            relevant_lines: location_a..=location_a,
        }
    }
}

impl ErrorLocation {
    pub fn get_line_idx(&self) -> Option<usize> {
        match self {
            ErrorLocation::NoLocation => None,
            ErrorLocation::LineOnly(x) => Some(*x),
            ErrorLocation::LineAndMeasure(x, _) => Some(*x),
            ErrorLocation::LineAndCharIdx(x, _) => Some(*x),
        }
    }
    pub fn write_location_explainer(&self, f: &mut impl std::fmt::Write) {
        match self {
            ErrorLocation::NoLocation => (),
            ErrorLocation::LineOnly(line_idx) => {
                let line_num = line_idx + 1;
                writeln!(f, "{} in line {line_num}:", "Where:".bold(),).unwrap();
            }
            ErrorLocation::LineAndMeasure(line_idx, measure_idx) => {
                let (line_num, measure_num) = (line_idx + 1, measure_idx + 1);
                writeln!(
                    f,
                    "{} Measure {measure_num} in line {line_num}:",
                    "Where:".bold()
                )
                .unwrap();
            }
            ErrorLocation::LineAndCharIdx(line_idx, char_idx) => writeln!(
                f,
                "{} line {} char {}",
                "Where:".bold(),
                line_idx + 1,
                char_idx + 1,
            )
            .unwrap(),
        }
    }
}

pub const ERROR_CONTEXT: usize = 3;

/// When reporting an error with `relevant_lines`, we want to show some context of +=, [ERROR_CONTEXT]
/// lines that are not neccessarily relevant
/// This is not always possible because of line bounds (cannot show 3 lines before line 0)
/// This function handles that.
pub fn extend_error_range(range: &RangeInclusive<usize>, line_cnt: usize) -> RangeInclusive<usize> {
    let start = max(*range.start(), ERROR_CONTEXT) - ERROR_CONTEXT;
    let end = min(line_cnt, range.end() + ERROR_CONTEXT);

    start..=end
}

pub fn get_digit_cnt(num: usize) -> u32 {
    num.checked_ilog10().unwrap_or(0) + 1
}
