#[derive(Debug)]
pub enum BackendErrorKind {
    IOError(std::io::Error),
    FmtError(std::fmt::Error),
    EmptyScore,
    // TODO: maybe this shouldn't be an error?
    NoClosingBarline,
    Parse3InvalidCharacter(char),
    FixupFailed,
    InvalidStringName,
}

impl BackendErrorKind {
    pub fn desc(&self) -> (String, String) {
        match self {
            BackendErrorKind::IOError(x) => {
                ("Cannot write to file".into(), format!("IO Error:\n{x}"))
            }

            BackendErrorKind::FmtError(x) => {
                ("Cannot write to internal buffer".into(), format!("Format error:\n{x}"))
            }
            BackendErrorKind::EmptyScore => ("Empty score".into(), String::new()),

            BackendErrorKind::NoClosingBarline => (
                "No closing barline".into(),
                "Lines in a part must end with a barline, but this one doesn't".into(),
            ),
            BackendErrorKind::Parse3InvalidCharacter(c) => {
                ("Invalid character".into(), format!("The character {c} is not valid here."))
            }
            BackendErrorKind::FixupFailed => (
                "Fixup failed".into(),
                "Failed to fix the error at this location after 5 tries".into(),
            ),
            BackendErrorKind::InvalidStringName => (
                "Invalid string name".into(),
                "Failed to parse the string name on this string".into(),
            ),
        }
    }
}
