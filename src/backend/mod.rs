use errors::{backend_error::BackendError, diagnostic::Diagnostic};

use crate::parser::parser2::ParserInput;
use std::{fmt::Display, time::Duration};
pub mod errors;
pub mod format;
pub mod midi;
pub mod muxml;
pub mod muxml2;
pub struct BackendResult<'a> {
    pub diagnostics: Vec<Diagnostic>,
    pub err: Option<BackendError<'a>>,
    pub timing_parse: Option<Duration>,
    pub timing_gen: Option<Duration>,
}
impl<'a> BackendResult<'a> {
    pub fn new(
        diagnostics: Vec<Diagnostic>,
        err: Option<BackendError<'a>>,
        timing_parse: Option<Duration>,
        timing_gen: Option<Duration>,
    ) -> Self {
        Self {
            diagnostics,
            err,
            timing_parse,
            timing_gen,
        }
    }
}
pub trait Backend {
    type BackendSettings;

    fn process<'a, Out: std::io::Write>(
        input: impl ParserInput<'a>,
        ou: &mut Out,
        settings: Self::BackendSettings,
    ) -> BackendResult<'a>;
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
    pub fn process<'a, Out: std::io::Write>(
        self,
        input: impl ParserInput<'a>,
        out: &'a mut Out,
    ) -> BackendResult {
        match self {
            BackendSelector::Midi(settings) => midi::MidiBackend::process(input, out, settings),
            BackendSelector::Muxml(settings) => muxml::MuxmlBackend::process(input, out, settings),
            BackendSelector::Muxml2(settings) => {
                muxml2::Muxml2Backend::process(input, out, settings)
            }
            BackendSelector::Format(settings) => {
                format::FormatBackend::process(input, out, settings)
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
