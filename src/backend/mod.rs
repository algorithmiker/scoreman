use errors::{backend_error::BackendError, diagnostic::Diagnostic};

use crate::parser::parser2::Parse2Result;
use std::fmt::Display;
pub mod errors;
pub mod format;
pub mod midi;
pub mod muxml;
pub mod muxml2;

pub trait Backend {
    type BackendSettings;

    /// A backend takes a Score, processes it to some format
    /// and writes the output to out.
    fn process<Out: std::io::Write>(
        parse_result: Parse2Result,
        ou: &mut Out,
        settings: Self::BackendSettings,
    ) -> Result<Vec<Diagnostic>, BackendError>;
}

/// Handles backend dispatch. Can be easily created from a string identifier
#[derive(Clone)]
pub enum BackendSelector {
    Midi(()),
    Muxml(()),
    Muxml2(muxml2::settings::Settings),
    Format(format::FormatBackendSettings),
}

impl BackendSelector {
    pub fn process<Out: std::io::Write>(
        self,
        parsed: Parse2Result,
        out: &mut Out,
    ) -> Result<Vec<Diagnostic>, BackendError> {
        match self {
            BackendSelector::Midi(settings) => midi::MidiBackend::process(parsed, out, settings),
            BackendSelector::Muxml(settings) => muxml::MuxmlBackend::process(parsed, out, settings),
            BackendSelector::Muxml2(settings) => {
                muxml2::Muxml2Backend::process(parsed, out, settings)
            }
            BackendSelector::Format(settings) => {
                format::FormatBackend::process(parsed, out, settings)
            }
        }
    }
}

impl Display for BackendSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BackendSelector::Midi(_) => "midi",
                BackendSelector::Muxml(_) => "muxml",
                BackendSelector::Muxml2(_) => "muxml",
                BackendSelector::Format(_) => "format",
            }
        )
    }
}
