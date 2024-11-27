use clap::ValueEnum;

/// These are documented in [cli_args]
#[derive(Clone)]
pub struct Settings {
    pub remove_rest_between_notes: bool,
    pub trim_measure: bool,
    pub simplify_time_signature: bool,
    pub bend_mode: Muxml2BendMode,
}

#[derive(ValueEnum, Clone)]
pub enum Muxml2BendMode {
    /// Use the <bend> element from the standard for bends. Some MusicXML viewers/sheet music editors do not support this, so you may want to use
    /// [EmulateBends] instead.
    StandardsCompliant,

    /// Emulate bends for editors that don't support them natively by using two notes and a slur between them.
    EmulateBends,
}
