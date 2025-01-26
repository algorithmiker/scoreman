pub mod fretboard;
pub mod muxml2_formatters;
#[cfg(test)]
mod muxml2_tests;
pub mod settings;

use crate::parser::parser3;
use crate::parser::parser3::{Parse3Result, TabElement3};
use crate::{
    backend::{
        errors::{
            backend_error::BackendError, backend_error_kind::BackendErrorKind,
            error_location::ErrorLocation,
        },
        Backend, BackendResult,
    },
    debugln, rlen, time, traceln,
};
use bilge::prelude::{u1, u2};
use fretboard::get_fretboard_note2;
use itertools::Itertools;
use muxml2_formatters::{
    write_muxml2_measure_prelude, write_muxml2_note, write_muxml2_rest, Slur, MUXML2_DOCUMENT_END,
    MUXML_INCOMPLETE_DOC_PRELUDE,
};
use settings::Muxml2BendMode;
use std::collections::HashMap;
use std::time::Duration;

pub struct Muxml2Backend();
impl Backend for Muxml2Backend {
    type BackendSettings = settings::Settings;

    fn process<'a, Out: std::io::Write>(
        input: &'a [String], out: &mut Out, settings: Self::BackendSettings,
    ) -> BackendResult<'a> {
        let (parse_time, parse_result) = time(|| parser3::parse3(input));
        match parse_result.error {
            None => {}
            Some(err) => return BackendResult::new(vec![], Some(err), Some(parse_time), None),
        }
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
    Rest(u32),
    CopyTick(u32),
    /// used in optimizing, should generate no code for this type
    Invalid,
}

impl Muxml2TabElement {
    fn write_muxml<A: std::fmt::Write>(
        &self, parsed: &Parse3Result, buf: &mut A, note_properties: &HashMap<u32, NoteProperties>,
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
            Muxml2TabElement::CopyTick(tick_idx) => {
                let note_range = *tick_idx as usize..=(*tick_idx as usize + 5);
                let notes_iter = parsed.tick_stream[note_range]
                    .iter()
                    .enumerate()
                    .filter(|x| !matches!(x.1, TabElement3::Rest))
                    .map(|(x, y)| (x + *tick_idx as usize, y));
                // at least two notes here
                let tick_chord = notes_iter.clone().take(2).count() == 2;
                traceln!(
                    "for CopyTick({tick_idx}): range {:?}, chord={tick_chord}",
                    *tick_idx as usize..=(*tick_idx as usize + 5)
                );
                // TODO: use dynamic base notes - we parse it but we don't use it
                let mut chord_first_written = false;
                for (elem_idx, elem) in notes_iter {
                    let need_chord = tick_chord && chord_first_written;
                    traceln!(depth = 1, "elem={elem:?}@{elem_idx} chord flag: {need_chord}");

                    match elem {
                        TabElement3::Fret(x) => {
                            chord_first_written = true;
                            let note =
                                get_fretboard_note2(parsed.base_notes[elem_idx % 6], *x).unwrap();
                            let (step, octave, sharp) = note.step_octave_sharp();
                            let properties = note_properties.get(&(elem_idx as u32));
                            write_muxml2_note(
                                buf, step, octave, sharp, need_chord, false, properties,
                            )?;
                        }
                        TabElement3::DeadNote => {
                            chord_first_written = true;
                            let note =
                                get_fretboard_note2(parsed.base_notes[elem_idx % 6], 0).unwrap();
                            let (step, octave, sharp) = note.step_octave_sharp();
                            let properties = note_properties.get(&(elem_idx as u32));
                            write_muxml2_note(
                                buf, step, octave, sharp, need_chord, true, properties,
                            )?;
                        }
                        TabElement3::Rest => unreachable!(),
                        TabElement3::Bend
                        | TabElement3::HammerOn
                        | TabElement3::Pull
                        | TabElement3::Release
                        | TabElement3::Slide
                        | TabElement3::Vibrato => {}
                    }
                }

                Ok(())
            }
            Muxml2TabElement::Invalid => Ok(()),
        }
    }
}

pub trait ToMuxml {
    fn write_muxml(
        &self, buf: &mut impl std::fmt::Write, string: char, chord: bool, slur_cnt: &mut u32,
        bend_mode: Muxml2BendMode, bend_target: &Option<&u8>,
    ) -> Result<(), std::fmt::Error>;
}
#[derive(Default, Debug)]
pub struct Slur2 {
    pub number: u16,
    pub start: bool,
}
impl Slur2 {
    pub fn new(number: u16, start: bool) -> Self {
        Slur2 { number, start }
    }
}
#[derive(Default, Debug)]
pub struct Slide2 {
    pub number: u16,
    pub start: bool,
}
impl Slide2 {
    pub fn new(number: u16, start: bool) -> Self {
        Slide2 { number, start }
    }
}
/// TODO: make this a bitstruct and see if that is faster
/// TODO: try making this a SoA
#[derive(Default, Debug)]
pub struct NoteProperties {
    pub slurs: Vec<Slur2>,
    pub slide: Option<Slide2>,
    pub vibrato: Option<Vibrato2>,
}
#[derive(Debug)]
pub enum Vibrato2 {
    Start,
    Stop,
}
fn gen_muxml2<'a>(
    parse_time: Duration, mut parsed: Parse3Result,
    settings: <Muxml2Backend as Backend>::BackendSettings,
) -> (Option<String>, BackendResult<'a>) {
    // status of the project:
    // parser3 is mostly done and works well and fast,
    // but the codegen backends need to be adapted
    // muxml2 especially, as it can be made much faster
    // since, especially with std::simd, comparing the next 6 ticks against a TabElem3::Rest should be trivial
    //     (with a splat-compare)

    let diagnostics = vec![];
    let number_of_measures = parsed.measures.len();
    let mut document = String::from(MUXML_INCOMPLETE_DOC_PRELUDE);
    let cap = MUXML_INCOMPLETE_DOC_PRELUDE.len()
        + MUXML2_DOCUMENT_END.len()
        + parsed.tick_stream.len() * 20;
    document.reserve(cap);
    debugln!("muxml2: reserved {}", cap);
    let mut slur_cnt = 0;
    let mut slide_count = 0;
    let mut note_properties: HashMap<u32, NoteProperties> = HashMap::new();
    for measure_idx in 0..number_of_measures {
        traceln!("Muxml2: processing measure {}", measure_idx);
        let ticks_in_measure = rlen(&parsed.measures[measure_idx].data_range) / 6; // see assumption 2
        debug_assert!(rlen(&parsed.measures[measure_idx].data_range) % 6 == 0);
        // Length of actual content in measure. `remove_space_between_notes` will reduce this for
        // example
        let mut measure_content_len = ticks_in_measure;
        debugln!("initial measure_content_len = {measure_content_len}");
        let mut measure_processed: Vec<Muxml2TabElement> =
            Vec::with_capacity(ticks_in_measure as usize);
        let mut stream_idx: usize = *parsed.measures[measure_idx].data_range.start() as usize;
        let mut note_count = 0;
        let mut stream_proc_cnt = 0;
        while stream_idx <= *parsed.measures[measure_idx].data_range.end() as usize {
            let elem = &parsed.tick_stream[stream_idx];
            //traceln!(
            //    depth = 2,
            //    "current elem: {elem:?}, note_count: {note_count}, proc_cnt = {stream_proc_cnt}"
            //);
            match elem {
                TabElement3::Fret(x) => {
                    note_count += 1;
                }
                TabElement3::Rest => {}
                TabElement3::DeadNote => {
                    note_count += 1;
                }
                TabElement3::Vibrato => {
                    let last_idx = stream_idx.saturating_sub(6);
                    note_properties.entry(last_idx as u32).or_default().vibrato =
                        Some(Vibrato2::Start);
                    let next_idx = stream_idx + 6;
                    if next_idx >= parsed.tick_stream.len() {
                        parsed.tick_stream.extend([const { TabElement3::Rest }; 6]);
                    }
                    note_properties.entry(next_idx as u32).or_default().vibrato =
                        Some(Vibrato2::Stop);
                }
                TabElement3::Bend
                | TabElement3::HammerOn
                | TabElement3::Pull
                | TabElement3::Release => {
                    // TODO: eventually mark hammerOns and pulls
                    // PRERELEASE: add errors for the panics here
                    let last_idx = stream_idx.saturating_sub(6);
                    traceln!(
                        "muxml2: have bend at tick {stream_idx}. last element on this string is (@{last_idx}): {:?}",
                        parsed.tick_stream[last_idx]
                    );
                    slur_cnt += 1;
                    note_properties
                        .entry(last_idx as u32)
                        .or_default()
                        .slurs
                        .push(Slur2::new(slur_cnt, true));
                    let next_idx = stream_idx + 6;

                    match &parsed.tick_stream.get(next_idx) {
                        None => {
                            traceln!("hanging bend on {stream_idx} at stream end");
                            if let TabElement3::Fret(x) = parsed.tick_stream[last_idx] {
                                parsed.tick_stream.extend([const { TabElement3::Rest }; 6]);
                                parsed.tick_stream[next_idx] = TabElement3::Fret(x + 1);

                                note_properties
                                    .entry(next_idx as u32)
                                    .or_default()
                                    .slurs
                                    .push(Slur2::new(slur_cnt, false));
                            } else {
                                panic!()
                            }
                        }
                        // hack: since we know that with a "hanging bend" the next element in this track is going to be a rest, we can just sliently replace it and add the correct note
                        Some(TabElement3::Rest) => {
                            if let TabElement3::Fret(x) = parsed.tick_stream[last_idx] {
                                parsed.tick_stream[next_idx] = TabElement3::Fret(x + 1);
                                traceln!("hanging bend on {stream_idx}, replacing {next_idx} with a Fret");
                                note_properties
                                    .entry(next_idx as u32)
                                    .or_default()
                                    .slurs
                                    .push(Slur2::new(slur_cnt, false));
                            } else {
                                panic!();
                            }
                        }
                        _ => {
                            note_properties
                                .entry(next_idx as u32)
                                .or_default()
                                .slurs
                                .push(Slur2::new(slur_cnt, false));
                        }
                    }

                    traceln!("added bend with start idx {last_idx} and end idx {next_idx}")
                }
                TabElement3::Slide => {
                    let last_idx = stream_idx.saturating_sub(6);
                    traceln!(
                        depth = 1,
                        "muxml2: have Slide. last element on this string is: {:?}",
                        parsed.tick_stream[last_idx]
                    );
                    slide_count += 1;
                    note_properties.entry(last_idx as u32).or_default().slide =
                        Some(Slide2::new(slide_count, true));
                    let next_idx = stream_idx + 6;
                    if next_idx < parsed.tick_stream.len() {
                        note_properties.entry(next_idx as u32).or_default().slide =
                            Some(Slide2::new(slide_count, false));
                    }
                    traceln!(
                        depth = 1,
                        "added slide with start idx {stream_idx} and end idx {next_idx}"
                    )
                }
            }
            stream_idx += 1;

            if stream_proc_cnt == 5 {
                stream_proc_cnt = 0;
                if note_count > 0 {
                    measure_processed.push(Muxml2TabElement::CopyTick(stream_idx as u32 - 6));
                // TODO: maybe pass the non-rest tick ids here instead?
                } else {
                    measure_processed.push(Muxml2TabElement::Rest(1));
                }
                traceln!(depth = 1, "Parsed a tick, a {:?} ", measure_processed.last().unwrap());
                note_count = 0;
            } else {
                stream_proc_cnt += 1;
            }
        }
        if settings.remove_rest_between_notes {
            remove_rest_between_notes(&mut measure_processed, &mut measure_content_len);
        }
        merge_rests_in_measure(&mut measure_processed);
        if settings.trim_measure {
            trim_measure(&mut measure_processed, &mut measure_content_len, Direction::Forward);
            trim_measure(&mut measure_processed, &mut measure_content_len, Direction::Backward);
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
            measure_enumerator as usize,
            measure_denominator,
        )
        .unwrap();
        for proc_elem in measure_processed {
            if let Err(x) = proc_elem.write_muxml(&parsed, &mut document, &note_properties) {
                return (None, BackendResult::new(diagnostics, Some(x.into()), None, None));
            }
        }
        document.push_str("</measure>");
    }

    document += MUXML2_DOCUMENT_END;
    debugln!("muxml2: document capacity on finish: {}", document.capacity());
    (Some(document), BackendResult::new(diagnostics, None, Some(parse_time), None))
}

fn merge_rests_in_measure(measure: &mut [Muxml2TabElement]) {
    for mut i in 0..measure.len() {
        match measure[i] {
            Muxml2TabElement::Rest(x) => {
                debug_assert_eq!(x, 1, "Expect Muxml2TabElement::Rest(1) in unprocessed AST, got Muxml2TabElement::Rest({x})");
                let original_i = i;
                while i < measure.len() && matches!(measure[i], Muxml2TabElement::Rest(1)) {
                    measure[i] = Muxml2TabElement::Invalid;
                    i += 1;
                }
                measure[original_i] = Muxml2TabElement::Rest((i - original_i) as u32);
            }
            Muxml2TabElement::CopyTick(..) | Muxml2TabElement::Invalid => continue,
        }
    }
}

fn remove_rest_between_notes(measure: &mut [Muxml2TabElement], content_len: &mut u32) {
    // remove rest between notes if wanted
    let mut i = 0;
    while i < measure.len() {
        use Muxml2TabElement::*;
        match (measure.get(i), measure.get(i + 1), measure.get(i + 2)) {
            (Some(CopyTick(_)), Some(Rest(1)), Some(CopyTick(_))) => {
                measure[i + 1] = Muxml2TabElement::Invalid;
                i += 3;
                *content_len -= 1;
            }
            (Some(Rest(1)), Some(CopyTick(_)), Some(Rest(1))) => {
                measure[i] = Muxml2TabElement::Invalid;
                measure[i + 2] = Muxml2TabElement::Invalid;
                i += 3;
                *content_len -= 2;
            }
            _ => {
                i += 1;
            }
        }
    }
}
enum Direction {
    Forward,
    Backward,
}

fn trim_measure(measure: &mut [Muxml2TabElement], content_len: &mut u32, direction: Direction) {
    let (mut i, last) = match direction {
        Direction::Forward => (0, measure.len() - 1),
        Direction::Backward => (measure.len() - 1, 0),
    };
    loop {
        match &measure[i] {
            Muxml2TabElement::Rest(rest_len) => {
                // TODO: this generates an empty measure for full-rest measures. maybe think of a solution
                *content_len = content_len.saturating_sub(*rest_len);
                measure[i] = Muxml2TabElement::Invalid;
                break;
            }
            Muxml2TabElement::CopyTick(_) => break,
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
}
