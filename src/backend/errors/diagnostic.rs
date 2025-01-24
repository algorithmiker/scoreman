use super::{diagnostic_kind::DiagnosticKind, error_location::ErrorLocation};
use std::fmt::{self, Display};
use yansi::Paint;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub location: ErrorLocation,
    pub kind: DiagnosticKind,
    pub severity: DiagnosticSeverity,
}

impl Diagnostic {
    pub fn info(location: ErrorLocation, kind: DiagnosticKind) -> Self {
        Self { location, kind, severity: DiagnosticSeverity::Info }
    }
    pub fn warn(location: ErrorLocation, kind: DiagnosticKind) -> Self {
        Self { location, kind, severity: DiagnosticSeverity::Warning }
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
