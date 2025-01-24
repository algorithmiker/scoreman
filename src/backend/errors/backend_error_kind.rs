#[derive(Debug)]
pub enum BackendErrorKind<'a> {
    IOError(std::io::Error),
    FmtError(std::fmt::Error),
    EmptyScore,
    /// string name and fret
    NoSuchFret(char, u8),
    TickMismatch(char, char, usize, usize),
    InvalidPartlineSyntax(&'a str),
    // TODO: maybe this shouldn't be an error?
    NoClosingBarline,
    Parse3InvalidCharacter(char),
    // TODO: a parser error for invalid string names
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
            BackendErrorKind::InvalidPartlineSyntax(rem) => ("Invalid partline syntax".into(), format!("Got remaining content: `{rem}`")),
            BackendErrorKind::NoClosingBarline => {("No closing barline".into(), "Lines in a part must end with a barline, but this one doesn't".into())}
            BackendErrorKind::Parse3InvalidCharacter(c) => {
                ("Invalid character".into(), format!("The character {c} is not valid here"))
            }
        }
    }
}
