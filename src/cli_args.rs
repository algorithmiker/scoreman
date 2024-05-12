use std::fmt::Display;

use clap::{Parser, Subcommand};
use guitar_tab::backend::{muxml2, BackendSelector};

#[derive(Parser)]
#[command(
    version,
    name = "guitar_tab",
    about = "Transforms a melody in guitar tab notation into a score in standard music notation"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// The most complex backend, usually produces the best results but is slower than the others
    /// and in more cases cannot work over imperfections of a bad tab
    #[command(visible_alias = "musicxml2", long_about = "")]
    Muxml2 {
        /// A lot of tabs will leave rest before/after the measure content for better clarity.
        /// This option will remove those.
        #[arg(short = 'm', long)]
        trim_measure: bool,

        /// A lot of tabs will leave a rest between each note, even when it is not needed to
        /// discriminate between single- and double-digit frets. This will remove these,
        /// effectively transforming the IR [1,rest,2,rest,3,rest,4,rest,5] of `e|1-2-3-4-5|`
        /// into [1,2,3,4,5]
        #[arg(short = 'n', long)]
        remove_rest_between_notes: bool,
        #[arg(short = 't', long)]
        /// Simplify time signature, e.g. 8/8 to 4/4
        simplify_time_signature: bool,
        input_path: String,
        output_path: String,
    },

    /// The older muxml backend. Needs a less perfect tab than Muxml2, but produces a multi-track
    /// document which is uglier and harder to work with
    #[command(visible_alias = "musicxml")]
    Muxml {
        input_path: String,
        output_path: String,
    },

    /// The simplest backend, creates a SMF file. Very fast, good for even realtime applications
    /// (usually runs in nanoseconds even for complex tabs), but importing into a score application
    /// will result in an even uglier score than muxml1.
    /// If you need a lot of speed, consider using the library directly (not via cli) because
    /// argument parsing adds ~100us
    Midi {
        input_path: String,
        output_path: String,
    },

    /// Formats the score into a new .tab annotated with measure indices. Also good for debugging
    /// scores. Does minimal parsing, so a score that can be formatted isn't neccessarily valid.
    Format {
        input_path: String,
        output_path: String,
    },
}

impl Commands {
    pub fn input_path(&self) -> &str {
        match self {
            Commands::Muxml2 { input_path, .. }
            | Commands::Muxml { input_path, .. }
            | Commands::Midi { input_path, .. }
            | Commands::Format { input_path, .. } => input_path,
        }
    }

    pub fn output_path(&self) -> &str {
        match self {
            Commands::Muxml2 { output_path, .. }
            | Commands::Muxml { output_path, .. }
            | Commands::Midi { output_path, .. }
            | Commands::Format { output_path, .. } => output_path,
        }
    }

    pub fn to_backend_selector(&self) -> BackendSelector {
        match self {
            Commands::Muxml2 {
                trim_measure,
                remove_rest_between_notes,
                simplify_time_signature,
                ..
            } => BackendSelector::Muxml2(muxml2::settings::Settings {
                remove_rest_between_notes: *remove_rest_between_notes,
                trim_measure: *trim_measure,
                simplify_time_signature: *simplify_time_signature,
            }),
            Commands::Muxml { .. } => BackendSelector::Muxml(()),
            Commands::Midi { .. } => BackendSelector::Midi(()),
            Commands::Format { .. } => BackendSelector::Format(()),
        }
    }
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Commands::Muxml2 { .. } => "muxml2",
                Commands::Muxml { .. } => "muxml",
                Commands::Midi { .. } => "midi",
                Commands::Format { .. } => "format",
            }
        )
    }
}
