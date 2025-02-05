use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    EmptyLineInPart,
    CommentInPart,
    FormatAddedBarline,
    FormatReplacedInvalid,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticKind::EmptyLineInPart => {
                write!(f, "Empty line inside Part, are you sure this is intended?")
            }
            DiagnosticKind::CommentInPart => {
                write!(f, "Comment inside Part, are you sure this is intended?")
            }
            DiagnosticKind::FormatAddedBarline => {
                write!(f, "There was no barline at the end of this line, so I added one.")
            }
            DiagnosticKind::FormatReplacedInvalid => {
                write!(f, "This character is invalid, so I replaced it with a rest (`-`).")
            }
        }
    }
}
