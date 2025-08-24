pub mod formatters;
pub mod fretboard;
#[cfg(test)]
mod muxml2_tests;
pub mod settings;
use crate::backend::errors::backend_error::BackendError;
use crate::parser::parser;
use crate::parser::parser::{source_location_from_stream, ParseResult};
use crate::parser::tab_element::TabElement;
use crate::{
    backend::{Backend, BackendResult},
    debugln, rlen, time, traceln,
};
use formatters::{
    write_muxml2_measure_prelude, write_muxml2_note, write_muxml2_rest, MUXML2_DOCUMENT_END,
    MUXML_INCOMPLETE_DOC_PRELUDE,
};
use fretboard::get_fretboard_note2;
use rustc_hash::FxBuildHasher;
use std::collections::HashMap;
use std::time::Duration;

pub struct MuxmlBackend();
impl Backend for MuxmlBackend {
    type BackendSettings = settings::Settings;

    fn process<Out: std::io::Write>(
        input: &[String], out: &mut Out, settings: Self::BackendSettings,
    ) -> BackendResult {
        let (parse_time, parse_result) = time(|| parser::parse(input));
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

#[inline(always)]
fn write_rest(buf: &mut impl std::fmt::Write, mut x: u32) -> std::fmt::Result {
    while x != 0 {
        for bound in [(8, "whole"), (4, "half"), (2, "quarter"), (1, "eighth")] {
            if x >= bound.0 {
                write_muxml2_rest(buf, bound.1, bound.0 as u8)?;
                x -= bound.0;
            }
        }
    }
    Ok(())
}

impl Muxml2TabElement {
    fn write_muxml<A: std::fmt::Write>(
        &self, parsed: &ParseResult, buf: &mut A,
        note_properties: &HashMap<u32, NoteProperties, impl std::hash::BuildHasher>,
    ) -> std::fmt::Result {
        match self {
            Muxml2TabElement::Rest(x) => write_rest(buf, *x),
            Muxml2TabElement::CopyTick(tick_idx) => {
                let note_range = *tick_idx as usize..=(*tick_idx as usize + 5);
                let notes_iter = parsed.tick_stream[note_range]
                    .iter()
                    .enumerate()
                    .filter(|x| !matches!(x.1, TabElement::Rest))
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
                    chord_first_written = true;

                    let Some((dead, fret)) = (match elem {
                        TabElement::DeadNote => Some((true, 0)),
                        TabElement::Fret(x) => Some((false, *x)),
                        _ => None,
                    }) else {
                        continue;
                    };
                    let string_name = parsed.base_notes[elem_idx % 6];
                    let note = get_fretboard_note2(string_name, fret).unwrap_or_else(|| {
                        panic!("Don't know base note for string name {string_name}",)
                    });
                    let (step, octave, sharp) = note.step_octave_sharp();
                    let properties = note_properties.get(&(elem_idx as u32));
                    write_muxml2_note(buf, step, octave, sharp, need_chord, dead, properties)?;
                }

                Ok(())
            }
            Muxml2TabElement::Invalid => Ok(()),
        }
    }
}

#[derive(Default, Debug)]
pub struct Slur {
    pub number: u16,
    pub start: bool,
}
impl Slur {
    pub fn new(number: u16, start: bool) -> Self {
        Slur { number, start }
    }
    pub fn start(number: u16) -> Self {
        Slur { number, start: true }
    }
    pub fn stop(number: u16) -> Self {
        Slur { number, start: false }
    }
}
#[derive(Default, Debug)]
pub struct Slide {
    pub number: u16,
    pub start: bool,
}
impl Slide {
    pub fn new(number: u16, start: bool) -> Self {
        Slide { number, start }
    }
}
/// TODO: make this a bitstruct and see if that is faster
/// TODO: try making this a SoA
#[derive(Default, Debug)]
pub struct NoteProperties {
    pub slurs: Vec<Slur>,
    pub slide: Option<Slide>,
    pub vibrato: Option<Vibrato>,
}
#[derive(Debug)]
pub enum Vibrato {
    Start,
    Stop,
}

fn gen_muxml2(
    parse_time: Duration, mut parsed: ParseResult,
    settings: <MuxmlBackend as Backend>::BackendSettings,
) -> (Option<String>, BackendResult) {
    let mut r = BackendResult::new(vec![], None, Some(parse_time), None);
    let number_of_measures = parsed.measures.len();
    let mut document = String::from(MUXML_INCOMPLETE_DOC_PRELUDE);
    let cap = MUXML_INCOMPLETE_DOC_PRELUDE.len()
        + MUXML2_DOCUMENT_END.len()
        + parsed.tick_stream.len() * 20;
    document.reserve(cap);
    debugln!("muxml2: reserved {}", cap);
    let mut slur_cnt = 0;
    let mut slide_count = 0;
    let mut note_properties: HashMap<u32, NoteProperties, FxBuildHasher> = HashMap::default();
    for measure_idx in 0..number_of_measures {
        traceln!("Muxml2: processing measure {}", measure_idx);
        let ticks_in_measure = rlen(&parsed.measures[measure_idx].data_range) / 6;
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
                TabElement::Fret(..) | TabElement::DeadNote => note_count += 1,
                TabElement::Rest => {}
                TabElement::Vibrato => {
                    let last_idx = stream_idx.saturating_sub(6) as u32;
                    note_properties.entry(last_idx).or_default().vibrato = Some(Vibrato::Start);
                    let next_idx = stream_idx + 6;
                    if next_idx >= parsed.tick_stream.len() {
                        parsed.tick_stream.extend([const { TabElement::Rest }; 6]);
                    }
                    note_properties.entry(next_idx as u32).or_default().vibrato =
                        Some(Vibrato::Stop);
                }
                TabElement::Bend
                | TabElement::HammerOn
                | TabElement::Pull
                | TabElement::Release => {
                    // TODO: eventually mark hammerOns and pulls
                    let last_idx = stream_idx.saturating_sub(6);
                    traceln!(
                        "muxml2: have bend at tick {stream_idx}. last element on this string is (@{last_idx}): {:?}",
                        parsed.tick_stream[last_idx]
                    );
                    slur_cnt += 1;
                    let idx32 = last_idx as u32;
                    note_properties.entry(idx32).or_default().slurs.push(Slur::start(slur_cnt));
                    let next_idx = stream_idx + 6;

                    match &parsed.tick_stream.get(next_idx) {
                        None => {
                            traceln!("hanging bend on {stream_idx} at stream end");
                            let TabElement::Fret(x) = parsed.tick_stream[last_idx] else {
                                let (line, char) =
                                    source_location_from_stream(&parsed, stream_idx as u32);
                                r.err = Some(BackendError::bend_on_invalid(line, char));
                                return (None, r);
                            };

                            parsed.tick_stream.extend([const { TabElement::Rest }; 6]);
                            parsed.tick_stream[next_idx] = TabElement::Fret(x + 1);
                            let entry = note_properties.entry(next_idx as u32).or_default();
                            entry.slurs.push(Slur::stop(slur_cnt));
                        }
                        // since we know that with a "hanging bend" the next element in this track is going to be a rest, we can just silently replace it and add the correct note
                        Some(TabElement::Rest) => {
                            let TabElement::Fret(x) = parsed.tick_stream[last_idx] else {
                                let (line, char) =
                                    source_location_from_stream(&parsed, stream_idx as u32);
                                r.err = Some(BackendError::bend_on_invalid(line, char));
                                return (None, r);
                            };
                            parsed.tick_stream[next_idx] = TabElement::Fret(x + 1);
                            traceln!(
                                "hanging bend on {stream_idx}, replacing {next_idx} with a Fret"
                            );
                            let entry = note_properties.entry(next_idx as u32).or_default();
                            entry.slurs.push(Slur::stop(slur_cnt));
                        }
                        _ => {
                            let entry = note_properties.entry(next_idx as u32).or_default();
                            entry.slurs.push(Slur::stop(slur_cnt));
                        }
                    }

                    traceln!("added bend with start idx {last_idx} and end idx {next_idx}")
                }
                TabElement::Slide => {
                    let last_idx = stream_idx.saturating_sub(6);
                    traceln!(
                        depth = 1,
                        "muxml2: have Slide. last element on this string is: {:?}",
                        parsed.tick_stream[last_idx]
                    );
                    slide_count += 1;
                    note_properties.entry(last_idx as u32).or_default().slide =
                        Some(Slide::new(slide_count, true));
                    let next_idx = stream_idx + 6;
                    if next_idx < parsed.tick_stream.len() {
                        note_properties.entry(next_idx as u32).or_default().slide =
                            Some(Slide::new(slide_count, false));
                    }
                    traceln!(
                        depth = 1,
                        "added slide with start idx {stream_idx} and end idx {next_idx}"
                    )
                }
            }
            stream_idx += 1;

            if stream_proc_cnt == 5 {
                if note_count > 0 {
                    measure_processed.push(Muxml2TabElement::CopyTick(stream_idx as u32 - 6));
                // TODO: maybe pass the non-rest tick ids here instead?
                } else {
                    measure_processed.push(Muxml2TabElement::Rest(1));
                }
                traceln!(depth = 1, "Parsed a tick, a {:?} ", measure_processed.last().unwrap());
                note_count = 0;
                stream_proc_cnt = 0;
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
                r.err = Some(x.into());
                return (None, r);
            }
        }
        document.push_str("</measure>");
    }

    document += MUXML2_DOCUMENT_END;
    debugln!("muxml2: document capacity on finish: {}", document.capacity());
    (Some(document), r)
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
