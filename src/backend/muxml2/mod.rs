pub mod fretboard;
pub mod muxml2_formatters;
#[cfg(test)]
mod muxml2_tests;
pub mod settings;

use std::{collections::HashMap, time::Duration};

use fretboard::get_fretboard_note2;
use muxml2_formatters::{
    write_muxml2_measure_prelude, write_muxml2_note, write_muxml2_rest, Slur, MUXML2_DOCUMENT_END,
    MUXML_INCOMPLETE_DOC_PRELUDE,
};
use settings::Muxml2BendMode;

use crate::{
    backend::errors::error_location::ErrorLocation,
    parser::{
        parser2::{Parse2Result, Parser2, ParserInput},
        TabElement::{self, Fret, Rest},
    },
    time,
};

use super::{
    errors::{backend_error::BackendError, backend_error_kind::BackendErrorKind},
    Backend, BackendResult,
};

pub struct Muxml2Backend();
impl Backend for Muxml2Backend {
    type BackendSettings = settings::Settings;

    fn process<'a, Out: std::io::Write>(
        input: impl ParserInput<'a>,
        out: &mut Out,
        settings: Self::BackendSettings,
    ) -> BackendResult<'a> {
        let parser = Parser2 {
            track_measures: true,
            track_sections: false,
        };
        let (parse_time, parse_result) = match time(|| parser.parse(input)) {
            (parse_time, Ok(parse_result)) => (parse_time, parse_result),
            (_, Err(err)) => return BackendResult::new(vec![], Some(err), None, None),
        };
        let (gen_time, (xml_out, mut gen_result)) =
            time(|| gen_muxml2(parse_time, parse_result, settings));
        gen_result.timing_gen = Some(gen_time);
        if gen_result.err.is_some() {
            return gen_result;
        }
        if let Err(x) = out.write_all(xml_out.unwrap().as_bytes()) {
            gen_result.err = Some(x.into());
        }
        gen_result
    }
}

#[derive(Debug)]
pub enum Muxml2TabElement {
    Rest(usize),
    Notes(Vec<(u8, usize)>),
    /// used in optimizing, should generate no code for this type
    Invalid,
}

impl Muxml2TabElement {
    fn write_muxml<A: std::fmt::Write>(
        &self,
        parse_result: &Parse2Result,
        buf: &mut A,
        slur_cnt: &mut u32,
        bend_mode: &Muxml2BendMode,
    ) -> std::fmt::Result {
        match self {
            Muxml2TabElement::Rest(mut x) => {
                while x != 0 {
                    if x >= 8 {
                        write_muxml2_rest(buf, "whole", 8)?;
                        x -= 8;
                    } else if x >= 4 {
                        write_muxml2_rest(buf, "half", 4)?;
                        x -= 4;
                    } else if x >= 2 {
                        write_muxml2_rest(buf, "quarter", 2)?;
                        x -= 2;
                    } else {
                        debug_assert_eq!(x, 1);
                        write_muxml2_rest(buf, "eighth", 1)?;
                        x -= 1;
                    }
                }
                Ok(())
            }
            Muxml2TabElement::Notes(notes) => {
                let mut need_chord = notes.len() > 1;
                for note_pos in notes.iter() {
                    parse_result.strings[note_pos.0 as usize][note_pos.1]
                        .element
                        .write_muxml(
                            buf,
                            parse_result.string_names[note_pos.0 as usize],
                            need_chord,
                            slur_cnt,
                            bend_mode.clone(),
                            &parse_result
                                .bend_targets
                                .get(&(note_pos.0, note_pos.1 as u32)),
                        )?;
                    need_chord = false;
                }
                Ok(())
            }
            Muxml2TabElement::Invalid => Ok(()),
        }
    }
}

pub trait ToMuxml {
    fn write_muxml(
        &self,
        buf: &mut impl std::fmt::Write,
        string: char,
        chord: bool,
        slur_cnt: &mut u32,
        bend_mode: Muxml2BendMode,
        bend_target: &Option<&u8>,
    ) -> Result<(), std::fmt::Error>;
}
impl ToMuxml for TabElement {
    fn write_muxml(
        &self,
        buf: &mut impl std::fmt::Write,
        string: char,
        chord: bool,
        slur_cnt: &mut u32,
        bend_mode: Muxml2BendMode,
        bend_target: &Option<&u8>,
    ) -> Result<(), std::fmt::Error> {
        match self {
            Fret(x) => {
                let note = get_fretboard_note2(string, *x).unwrap();
                let (step, octave, sharp) = note.step_octave_sharp();
                write_muxml2_note(buf, step, octave, sharp, chord, false, Slur::None)?;
            }
            TabElement::FretBend(x) => {
                let note = get_fretboard_note2(string, *x).unwrap();
                let (step, octave, sharp) = note.step_octave_sharp();
                let slur = Slur::Start(bend_mode.clone(), *slur_cnt, 1);
                write_muxml2_note(buf, step, octave, sharp, chord, false, slur)?;

                let note = get_fretboard_note2(string, x + 1).unwrap();
                let (step, octave, sharp) = note.step_octave_sharp();
                let slur = Slur::End(bend_mode.clone(), *slur_cnt);
                write_muxml2_note(buf, step, octave, sharp, chord, false, slur)?;
                *slur_cnt += 1;
            }
            TabElement::FretBendTo(x) => {
                let y = bend_target.expect("FretBendTo without bend target");
                let note = get_fretboard_note2(string, *x).unwrap();
                let (step, octave, sharp) = note.step_octave_sharp();
                let slur = Slur::Start(bend_mode.clone(), *slur_cnt, *y as i8 - *x as i8);
                write_muxml2_note(buf, step, octave, sharp, chord, false, slur)?;

                let note = get_fretboard_note2(string, *y).unwrap();
                let (step, octave, sharp) = note.step_octave_sharp();
                let slur = Slur::End(bend_mode.clone(), *slur_cnt);
                write_muxml2_note(buf, step, octave, sharp, chord, false, slur)?;

                *slur_cnt += 1;
            }
            Rest => write_muxml2_rest(buf, "eighth", 1)?,
            TabElement::DeadNote => {
                let note = get_fretboard_note2(string, 0).unwrap();
                let (step, octave, sharp) = note.step_octave_sharp();
                write_muxml2_note(buf, step, octave, sharp, chord, false, Slur::None)?;
            }
        }
        Ok(())
    }
}

fn gen_muxml2<'a>(
    parse_time: Duration,
    parse_result: Parse2Result,
    settings: <Muxml2Backend as Backend>::BackendSettings,
) -> (Option<String>, BackendResult<'a>) {
    // the muxml2 backend assumes
    // 1. that there are the same number of measures for every string (which should be true)
    // 2. that there are the same number of ticks in the same measure for each string (also
    //    generally true)
    let diagnostics = vec![];
    let number_of_measures = parse_result.measures[0].len();
    let mut document = String::from(MUXML_INCOMPLETE_DOC_PRELUDE);
    // this looks like a good setting for -nmt based on trial and error
    document.reserve(parse_result.strings[0].len() * 6 * 10);
    //println!("Reserved capacity: {}", document.capacity());
    let mut slur_count = 0;
    for measure_idx in 0..number_of_measures {
        let ticks_in_measure = parse_result.measures[0][measure_idx].len(); // see assumption 2

        // Length of actual content in measure. `remove_space_between_notes` will reduce this for
        // example
        let mut measure_content_len = ticks_in_measure;
        let mut measure_processed: Vec<Muxml2TabElement> = vec![];
        for tick in 0..ticks_in_measure {
            // this was benchmarked and found to be
            // faster than a [MuxmlNote2;6]
            let mut notes_in_tick = Vec::with_capacity(6);
            for string_idx in 0..6 {
                let Some(raw_tick) = parse_result.measures[string_idx][measure_idx]
                    .get_content(&parse_result.strings[string_idx])
                    .get(tick)
                else {
                    let err = _tick_mismatch_err(parse_result, string_idx, measure_idx);
                    return (
                        None,
                        BackendResult::new(diagnostics, Some(err), Some(parse_time), None),
                    );
                };
                use TabElement::*;
                match raw_tick.element {
                    Fret(..) | DeadNote => {
                        // TODO: not sure if cloning would be faster here
                        notes_in_tick.push((
                            string_idx as u8,
                            parse_result.measures[string_idx][measure_idx]
                                .content
                                .start()
                                + tick,
                        ));
                    }
                    FretBend(..) | FretBendTo(..) => {
                        notes_in_tick.push((
                            string_idx as u8,
                            parse_result.measures[string_idx][measure_idx]
                                .content
                                .start()
                                + tick,
                        ));
                        // fix content len for stuff where we generate 2 notes for a bend
                        measure_content_len += 1;
                    }
                    _ => (),
                }
            }
            // if there were no notes inserted in this tick, add a rest
            measure_processed.push(if notes_in_tick.is_empty() {
                Muxml2TabElement::Rest(1)
            } else {
                Muxml2TabElement::Notes(notes_in_tick)
            })
        }

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

        // Try to simplify e.g 8/8 to 4/4
        let (mut measure_enumerator, mut measure_denominator) = (measure_content_len, 8);
        if settings.simplify_time_signature && measure_content_len % 2 == 0 {
            measure_enumerator /= 2;
            measure_denominator /= 2;
        }
        write_muxml2_measure_prelude(
            &mut document,
            measure_idx,
            measure_enumerator,
            measure_denominator,
        )
        .unwrap();
        for proc_elem in measure_processed {
            if let Err(x) = proc_elem.write_muxml(
                &parse_result,
                &mut document,
                &mut slur_count,
                &settings.bend_mode,
            ) {
                return (
                    None,
                    BackendResult::new(diagnostics, Some(x.into()), None, None),
                );
            }
        }
        document.push_str("</measure>");
    }

    document += MUXML2_DOCUMENT_END;
    //println!("Actual len: {}", document.len());
    (
        Some(document),
        BackendResult::new(diagnostics, None, Some(parse_time), None),
    )
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

const NOTE2_STEPS: [(char, bool); 12] = [
    ('C', false),
    ('C', true),
    ('D', false),
    ('D', true),
    ('E', false),
    ('F', false),
    ('F', true),
    ('G', false),
    ('G', true),
    ('A', false),
    ('A', true),
    ('B', false),
];
#[derive(Debug)]
pub struct MuxmlNote2 {
    /// Numeric representation of the frequency.
    ///
    /// step=0 is an octave 0 C,
    /// step=1 is an octave 0 C#,
    /// step=2 is an octave 0 D,
    /// and so on.
    ///
    /// Can represent 20 full octaves which should be plenty.
    pub step: u8,
    pub dead: bool,
}
impl MuxmlNote2 {
    pub fn step_octave_sharp(&self) -> (char, u8, bool) {
        let stepidx = (self.step % 12) as usize;
        let octave = self.step / 12;
        (NOTE2_STEPS[stepidx].0, octave, NOTE2_STEPS[stepidx].1)
    }
    pub fn write_muxml(
        &self,
        buf: &mut impl std::fmt::Write,
        chord: bool,
    ) -> Result<(), std::fmt::Error> {
        let (step, octave, sharp) = self.step_octave_sharp();
        write_muxml2_note(buf, step, octave, sharp, chord, self.dead, Slur::None)
    }
}

fn _tick_mismatch_err(
    parse_result: Parse2Result,
    string_idx: usize,
    measure_idx: usize,
) -> BackendError<'static> {
    let before_measure = &parse_result.measures[string_idx - 1][measure_idx];
    let this_measure = &parse_result.measures[string_idx][measure_idx];

    BackendError {
        main_location: ErrorLocation::LineAndMeasure(
            this_measure.parent_line,
            this_measure.index_on_parent_line,
        ),
        relevant_lines: before_measure.parent_line..=this_measure.parent_line,
        kind: BackendErrorKind::TickMismatch(
            parse_result.string_names[string_idx - 1],
            parse_result.string_names[string_idx],
            before_measure.len(),
            this_measure.len(),
        ),
    }
}
