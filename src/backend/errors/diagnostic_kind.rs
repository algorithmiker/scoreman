use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    Muxml1IsBad,
    Muxml1SeperateTracks,
    EmptyLineInPart,
    CommentInPart,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticKind::Muxml1IsBad => write!(f, "The MUXML1 backend is significantly worse than the MUXML2 backend. If you don't have any reason not to, use the MUXML2 backend"),
            DiagnosticKind::Muxml1SeperateTracks => write!(f,
              r#"The 6 strings of the guitar are labelled as separate instruments. To fix that,
                   1. import the generated file into MuseScore
                   2. select all tracks, do Tools->Implode
                   3. delete all other tracks except the first."#
            ),
            DiagnosticKind::EmptyLineInPart => write!(f, "Empty line inside Part, are you sure this is intended?"),
            DiagnosticKind::CommentInPart => write!(f, "Comment inside Part, are you sure this is intended?"),
        }
    }
}
