use crate::{collect_parse_error, parser::TabElement};

#[derive(Debug)]
pub enum BackendErrorKind<'a> {
    IOError(std::io::Error),
    FmtError(std::fmt::Error),
    EmptyScore,
    /// string name and fret
    NoSuchFret(char, u8),
    TickMismatch(char, char, usize, usize),
    /// string name where tick is multichar, string name here, tick idx, and the found invalid fret
    BadMulticharTick {
        /// string and fret
        multichar: (char, u8),
        /// something else
        invalid: (char, TabElement),
        tick_idx: usize,
    },
    InvalidPartlineSyntax(&'a str),
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
            BackendErrorKind::BadMulticharTick { multichar : (multichar_string,multichar_fret), invalid: (invalid_string,invalid_elem), tick_idx } =>
            (
                "Invalid multichar tick".into(),
                format!(
"Tick {} has a multi-char fret ({multichar_fret}) on string {multichar_string}, but on the same tick there is an invalid element {invalid_elem:?} on string {invalid_string}", tick_idx+1)
            ),
            BackendErrorKind::InvalidPartlineSyntax(rem) => ("Invalid partline syntax".into(), format!("Got remaining content: `{rem}`")),
            BackendErrorKind::ParseError(x) => ("Invalid syntax".into(), collect_parse_error(x)),

        }
    }
}
