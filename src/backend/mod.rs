use errors::{backend_error::BackendError, diagnostic::Diagnostic};

use std::{fmt::Display, time::Duration};
pub mod errors;
pub mod fixup;
pub mod midi;
pub mod muxml;
pub mod muxml2;
pub struct BackendResult {
    pub diagnostics: Vec<Diagnostic>,
    pub err: Option<BackendError>,
    pub timing_parse: Option<Duration>,
    pub timing_gen: Option<Duration>,
}
impl BackendResult {
    pub fn new(
        diagnostics: Vec<Diagnostic>, err: Option<BackendError>, timing_parse: Option<Duration>,
        timing_gen: Option<Duration>,
    ) -> Self {
        Self { diagnostics, err, timing_parse, timing_gen }
    }
}
pub trait Backend {
    type BackendSettings;

    fn process<Out: std::io::Write>(
        input: &[String], out: &mut Out, settings: Self::BackendSettings,
    ) -> BackendResult;
}

/// Handles backend dispatch. Can be easily created from a string identifier
#[derive(Clone)]
pub enum BackendSelector {
    Midi(()),
    // Muxml(()),
    Muxml2(muxml2::settings::Settings),
    Fixup(fixup::FixupBackendSettings),
}

impl BackendSelector {
    // TODO: remove the lifetime here
    pub fn process<Out: std::io::Write>(self, input: &[String], out: &mut Out) -> BackendResult {
        match self {
            BackendSelector::Midi(settings) => midi::MidiBackend::process(input, out, settings),
            // BackendSelector::Muxml(settings) => muxml::MuxmlBackend::process(input, out, settings),
            BackendSelector::Muxml2(settings) => {
                muxml2::Muxml2Backend::process(input, out, settings)
            }
            BackendSelector::Fixup(settings) => fixup::FixupBackend::process(input, out, settings),
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
                // BackendSelector::Muxml(_) => "muxml",
                BackendSelector::Muxml2(_) => "muxml",
                BackendSelector::Fixup(_) => "fixup",
            }
        )
    }
}
