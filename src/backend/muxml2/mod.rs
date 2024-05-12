pub mod fretboard;
mod muxml2_formatters;
#[cfg(test)]
mod muxml2_tests;
pub mod settings;
use anyhow::Context;

use crate::{
    parser::{
        Score,
        TabElement::{Fret, Rest},
    },
    raw_tracks::RawTracks,
};

use self::{
    fretboard::get_fretboard_note,
    muxml2_formatters::{muxml2_document, muxml2_measure, muxml2_note, muxml2_rest},
};

use super::Backend;
pub struct Muxml2Backend();
impl Backend for Muxml2Backend {
    type BackendSettings = settings::Settings;
    fn process<Out: std::io::Write>(
        score: Score,
        out: &mut Out,
        settings: Self::BackendSettings,
    ) -> anyhow::Result<()> {
        let raw_tracks = score.gen_raw_tracks()?;
        let xml_out = raw_tracks_to_muxml2(raw_tracks, settings)?;
        out.write_all(xml_out.as_bytes())?;
        println!("[I]: MUXML2 backend: Generated an Uncompressed MusicXML (.musicxml) file");
        Ok(())
    }
}

#[derive(Debug)]
enum Muxml2TabElement {
    Rest(usize),
    Notes(Vec<MuxmlNote>),
    /// used in optimizing, should generate no code for this type
    Invalid,
}

impl Muxml2TabElement {
    fn write_muxml<A: std::fmt::Write>(&self, buf: &mut A) -> anyhow::Result<()> {
        match self {
            Muxml2TabElement::Rest(mut x) => {
                while x != 0 {
                    if x >= 8 {
                        buf.write_str(&muxml2_rest("whole", 8))?;
                        x -= 8;
                    } else if x >= 4 {
                        buf.write_str(&muxml2_rest("half", 4))?;
                        x -= 4;
                    } else if x >= 2 {
                        buf.write_str(&muxml2_rest("quarter", 2))?;
                        x -= 2;
                    } else {
                        debug_assert_eq!(x, 1);
                        buf.write_str(&muxml2_rest("eighth", 1))?;
                        x -= 1;
                    }
                }
                Ok(())
            }
            Muxml2TabElement::Notes(notes) => {
                for (i, note) in notes.iter().enumerate() {
                    buf.write_str(&note.into_muxml("eighth", i != 0))?;
                }
                Ok(())
            }
            Muxml2TabElement::Invalid => Ok(()),
        }
    }
}

fn raw_tracks_to_muxml2(
    raw_tracks: RawTracks,
    settings: <Muxml2Backend as Backend>::BackendSettings,
) -> anyhow::Result<String> {
    // the muxml2 backend assumes
    // 1. that there are the same number of measures for every string (which should be true)
    // 2. that there are the same number of elements in the same measure for each string (also
    //    generally true)
    let number_of_measures = raw_tracks.1[0].len();
    let mut measures_xml = String::new();
    for measure_idx in 0..number_of_measures {
        let ticks_in_measure = raw_tracks.1[0][measure_idx].content.len();

        // Length of actual content in measure. `remove_space_between_notes` will reduce this for
        // example
        let mut measure_content_len = ticks_in_measure;
        let mut measure_processed: Vec<Muxml2TabElement> = vec![];
        for tick in 0..ticks_in_measure {
            let mut notes_in_tick = vec![];
            for string_idx in 0..6 {
                let note = &raw_tracks.1[string_idx][measure_idx]
                    .content
                    .get(tick)
                    .with_context(|| {
                        format_tick_mismatch_err(&raw_tracks, string_idx, measure_idx)
                    })?;
                match note {
                    Fret(x) => {
                        notes_in_tick.push(get_fretboard_note(raw_tracks.0[string_idx], *x)
                            .with_context(|| format!("Failed to get note for fret {x} on string {}, found in measure {measure_idx}", raw_tracks.0[string_idx]))?)
                    }
                    Rest => continue,
                }
            }
            // if there were no notes inserted in this tick, add a rest
            measure_processed.push(if notes_in_tick.is_empty() {
                Muxml2TabElement::Rest(1)
            } else {
                Muxml2TabElement::Notes(notes_in_tick)
            })
        }
        //println!("[D]: Measure before opt: {measure_processed:?}");

        // remove rest between notes if wanted
        if settings.remove_rest_between_notes {
            let mut i = 0;
            while i < measure_processed.len() {
                use Muxml2TabElement::*;
                match (
                    measure_processed.get(i),
                    measure_processed.get(i + 1),
                    measure_processed.get(i + 2),
                ) {
                    (Some(Notes(_)), Some(Rest(1)), Some(Notes(_))) => {
                        measure_processed[i + 1] = Muxml2TabElement::Invalid;
                        i += 3;
                        measure_content_len -= 1;
                    }
                    (Some(Rest(1)), Some(Notes(_)), Some(Rest(1))) => {
                        measure_processed[i] = Muxml2TabElement::Invalid;
                        measure_processed[i + 2] = Muxml2TabElement::Invalid;
                        i += 3;
                        measure_content_len -= 2;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
        }

        // merge rests in measure
        for mut i in 0..measure_processed.len() {
            match measure_processed[i] {
                Muxml2TabElement::Rest(x) => {
                    debug_assert_eq!(x, 1, "Expect Muxml2TabElement::Rest(1) in unprocessed AST, got Muxml2TabElement::Rest({x})");
                    let original_i = i;
                    while i < measure_processed.len()
                        && matches!(measure_processed[i], Muxml2TabElement::Rest(1))
                    {
                        measure_processed[i] = Muxml2TabElement::Invalid;
                        i += 1;
                    }
                    measure_processed[original_i] = Muxml2TabElement::Rest(i - original_i);
                }
                Muxml2TabElement::Notes(..) | Muxml2TabElement::Invalid => continue,
            }
        }

        if settings.trim_measure {
            trim_measure(
                &mut measure_processed,
                &mut measure_content_len,
                Direction::Forward,
            );
            trim_measure(
                &mut measure_processed,
                &mut measure_content_len,
                Direction::Backward,
            );
        }

        // write the final contents into a buffer
        let mut measure_xml = String::new();
        for proc_elem in measure_processed {
            proc_elem.write_muxml(&mut measure_xml)?;
        }

        // Try to simplify e.g 8/8 to 4/4
        let (mut measure_enumerator, mut measure_denominator) = (measure_content_len, 8);
        if settings.simplify_time_signature && measure_content_len % 2 == 0 {
            measure_enumerator /= 2;
            measure_denominator /= 2;
        }
        measures_xml += &muxml2_measure(
            measure_idx,
            measure_enumerator,
            measure_denominator,
            &measure_xml,
        );
    }

    Ok(muxml2_document(&measures_xml))
}

enum Direction {
    Forward,
    Backward,
}

fn trim_measure(measure: &mut [Muxml2TabElement], content_len: &mut usize, direction: Direction) {
    let (mut i, last) = match direction {
        Direction::Forward => (0, measure.len() - 1),
        Direction::Backward => (measure.len() - 1, 0),
    };
    loop {
        match &measure[i] {
            Muxml2TabElement::Rest(rest_len) => {
                *content_len -= rest_len;
                measure[i] = Muxml2TabElement::Invalid;
                break;
            }
            Muxml2TabElement::Notes(_) => break,
            Muxml2TabElement::Invalid => {
                if i == last {
                    break;
                }
                match direction {
                    Direction::Forward => i += 1,
                    Direction::Backward => i -= 1,
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct MuxmlNote {
    pub step: char,
    pub octave: u32,
    pub sharp: bool,
}

impl MuxmlNote {
    #[allow(clippy::wrong_self_convention)]
    pub fn into_muxml(&self, duration: &str, chord: bool) -> String {
        muxml2_note(self.step, self.octave, self.sharp, duration, chord)
    }
}

fn format_tick_mismatch_err(
    raw_tracks: &RawTracks,
    string_idx: usize,
    measure_idx: usize,
) -> String {
    let before_measure = &raw_tracks.1[string_idx - 1][measure_idx];
    let this_measure = &raw_tracks.1[string_idx][measure_idx];
    let explainer = match (before_measure.parent_line, this_measure.parent_line) {
        (Some(pbefore), Some(phere)) => {
            let before_line_num = pbefore + 1;
            let this_line_num = phere + 1;
            let (string_before, measure_before) = (
                raw_tracks.0[string_idx - 1],
                raw_tracks.1[string_idx - 1][measure_idx].print_pretty_string(),
            );
            let (string_here, measure_here) = (
                raw_tracks.0[string_idx],
                raw_tracks.1[string_idx][measure_idx].print_pretty_string(),
            );

            format!(
                "\nline {before_line_num}: {string_before}|{measure_before}|
line {this_line_num}: {string_here}|{measure_here}|\n"
            )
        }
        _ => String::new(),
    };
    format!("Problem in {measure_number}th measure:
The muxml2 backend relies on the fact that there are the same number of ticks (frets/rests) on every line (string) of a measure in the tab. This is not true for this tab.
{explainer}
[I] Tip 1: If you get a lot of errors like this, consider using the muxml1 backend.
[I] Tip 2: Use the format backend to check the contents of measure {measure_number}.",
measure_number = measure_idx +1)
}
