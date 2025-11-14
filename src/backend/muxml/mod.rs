pub mod formatters;
pub mod fretboard;
#[cfg(test)]
mod muxml2_tests;
pub mod settings;
use crate::backend::errors::backend_error::BackendError;
use crate::parser::tab_element::TabElement;
use crate::parser::{source_location_from_stream, ParseResult};
use crate::{
    backend::{Backend, BackendResult},
    rlen, time,
};
use crate::{parser, BufLines};
use formatters::{
    write_muxml2_measure_prelude, write_muxml2_note, write_muxml2_rest, MUXML2_DOCUMENT_END,
    MUXML_INCOMPLETE_DOC_PRELUDE,
};
use fretboard::get_fretboard_note2;
use rustc_hash::FxBuildHasher;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, span, trace, Level};

pub struct MuxmlBackend();
impl Backend for MuxmlBackend {
    type BackendSettings = settings::Settings;

    fn process<Out: std::io::Write>(
        input: &BufLines, out: &mut Out, settings: Self::BackendSettings,
    ) -> BackendResult {
        let (parse_time, parse_result) = time(|| parser::parse(input));
        if let Some(err) = parse_result.error {
            return BackendResult::new(vec![], Some(err), Some(parse_time), None);
        }
        let generator = MuxmlGenerator::init(parse_result, parse_time, settings);
        let (gen_time, (xml_out, mut gen_result)) = time(|| generator.gen());
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
    pub fn start(number: u16) -> Self {
        Slide { number, start: true }
    }
    pub fn stop(number: u16) -> Self {
        Slide { number, start: false }
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
pub struct MuxmlGenerator {
    parsed: ParseResult,
    settings: settings::Settings,

    measure_buf: Vec<Muxml2TabElement>,
    document: String,
    slur_cnt: u16,
    slide_cnt: u16,
    note_properties: HashMap<u32, NoteProperties, FxBuildHasher>,
    r: BackendResult,
}
impl MuxmlGenerator {
    pub fn estimate_capacity(parsed: &ParseResult) -> usize {
        // TODO: this is technically wrong, as .reserve reserves *additional* capacity.
        // Should fix this, but that needs readjusting the *20 multiplier.
        MUXML_INCOMPLETE_DOC_PRELUDE.len()
            + MUXML2_DOCUMENT_END.len()
            + parsed.tick_stream.len() * 20
    }
    /// Allocates heavily.
    pub fn init(parsed: ParseResult, parse_time: Duration, settings: settings::Settings) -> Self {
        let cap = Self::estimate_capacity(&parsed);
        let mut document = String::from(MUXML_INCOMPLETE_DOC_PRELUDE);
        document.reserve(cap);
        debug!(capacity = cap, "reserved capacity");

        Self {
            parsed,
            settings,
            document,
            slur_cnt: 0,
            slide_cnt: 0,
            note_properties: HashMap::default(),
            r: BackendResult::new(vec![], None, Some(parse_time), None),
            measure_buf: vec![],
        }
    }
    #[inline(always)]
    pub fn gen(mut self) -> (Option<String>, BackendResult) {
        let number_of_measures = self.parsed.measures.len();
        for measure_idx in 0..number_of_measures {
            if let Err(err) = self.process_measure(measure_idx) {
                self.r.err = Some(err);
                return (None, self.r);
            }
        }

        self.document += MUXML2_DOCUMENT_END;
        debug!(cap = self.document.capacity(), "document capacity on finish",);
        (Some(self.document), self.r)
    }

    #[inline(always)]
    pub fn start_slur(&mut self, idx: u32) {
        self.slur_cnt += 1;
        self.note_properties.entry(idx).or_default().slurs.push(Slur::start(self.slur_cnt));
    }
    #[inline(always)]
    pub fn stop_slur(&mut self, idx: u32) {
        self.note_properties.entry(idx).or_default().slurs.push(Slur::stop(self.slur_cnt));
    }
    #[inline(always)]
    pub fn start_slide(&mut self, idx: u32) {
        self.slide_cnt += 1;
        self.note_properties.entry(idx).or_default().slide = Some(Slide::start(self.slide_cnt));
    }
    #[inline(always)]
    pub fn stop_slide(&mut self, idx: u32) {
        self.note_properties.entry(idx).or_default().slide = Some(Slide::stop(self.slide_cnt));
    }
    #[inline(always)]
    pub fn start_vibrato(&mut self, idx: u32) {
        self.note_properties.entry(idx).or_default().vibrato = Some(Vibrato::Start);
    }
    #[inline(always)]
    pub fn stop_vibrato(&mut self, idx: u32) {
        self.note_properties.entry(idx).or_default().vibrato = Some(Vibrato::Stop);
    }
    pub fn process_bend_like(&mut self, stream_idx: usize) -> Result<(), BackendError> {
        // TODO: eventually mark hammerOns and pulls
        let last_idx = stream_idx.saturating_sub(6);
        trace!(stream_idx, "muxml2: have bend at tick {stream_idx}. last element on this string is (@{last_idx}): {:?}", self.parsed.tick_stream[last_idx]);
        self.start_slur(last_idx as u32);
        let next_idx = stream_idx + 6;

        match &self.parsed.tick_stream.get(next_idx) {
            None => {
                trace!(stream_idx, "hanging bend on at stream end");
                let TabElement::Fret(x) = self.parsed.tick_stream[last_idx] else {
                    let (line, char) = source_location_from_stream(&self.parsed, stream_idx as u32);
                    return Err(BackendError::bend_on_invalid(line, char));
                };

                self.parsed.tick_stream.extend([const { TabElement::Rest }; 6]);
                self.parsed.tick_stream[next_idx] = TabElement::Fret(x + 1);
                self.stop_slur(next_idx as u32);
            }
            // since we know that with a "hanging bend" the next element in this track is going to be a rest,
            // we can just silently replace it and add the correct note
            Some(TabElement::Rest) => {
                let TabElement::Fret(x) = self.parsed.tick_stream[last_idx] else {
                    let (line, char) = source_location_from_stream(&self.parsed, stream_idx as u32);
                    return Err(BackendError::bend_on_invalid(line, char));
                };
                self.parsed.tick_stream[next_idx] = TabElement::Fret(x + 1);
                trace!("hanging bend on {stream_idx}, replacing {next_idx} with a Fret");
                self.stop_slur(next_idx as u32);
            }
            _ => self.stop_slur(next_idx as u32),
        }

        trace!(start_idx = last_idx, end_idx = next_idx, "added bend");
        Ok(())
    }
    pub fn process_measure(&mut self, measure_idx: usize) -> Result<(), BackendError> {
        let meas = span!(Level::TRACE, "Muxml2: processing measure", measure_idx);
        let _meas = meas.enter();

        let data_range = &self.parsed.measures[measure_idx].data_range;
        let ticks_in_measure = rlen(data_range) / 6;
        debug_assert!(rlen(data_range).is_multiple_of(6));

        // Length of actual content in measure. `remove_space_between_notes` will reduce this for example
        let mut measure_content_len = ticks_in_measure;
        self.measure_buf.clear();
        debug!("initial measure_content_len = {measure_content_len}");

        let mut stream_idx: usize = *data_range.start() as usize;
        let (mut note_count, mut stream_proc_cnt) = (0, 0);
        let end = *data_range.end() as usize;
        while stream_idx <= end {
            let elem = &self.parsed.tick_stream[stream_idx];
            //trace!(
            //    "current elem: {elem:?}, note_count: {note_count}, proc_cnt = {stream_proc_cnt}"
            //);
            match elem {
                TabElement::Fret(..) | TabElement::DeadNote => note_count += 1,
                TabElement::Rest => {}
                TabElement::Vibrato => {
                    self.start_vibrato(stream_idx.saturating_sub(6) as u32);
                    let next_idx = stream_idx + 6;
                    if next_idx >= self.parsed.tick_stream.len() {
                        self.parsed.tick_stream.extend([const { TabElement::Rest }; 6]);
                    }
                    self.stop_vibrato(next_idx as u32);
                }
                TabElement::Bend
                | TabElement::HammerOn
                | TabElement::Pull
                | TabElement::Release => self.process_bend_like(stream_idx)?,
                TabElement::Slide => {
                    let last_idx = stream_idx.saturating_sub(6);
                    trace!(
                        "have Slide. last element on this string is: {:?}",
                        self.parsed.tick_stream[last_idx]
                    );
                    self.start_slide(last_idx as u32);
                    let next_idx = stream_idx + 6;
                    if next_idx < self.parsed.tick_stream.len() {
                        self.stop_slide(next_idx as u32);
                    }
                    trace!(start_idx = stream_idx, end_idx = next_idx, "added slide")
                }
            }
            stream_idx += 1;

            if stream_proc_cnt == 5 {
                if note_count > 0 {
                    self.measure_buf.push(Muxml2TabElement::CopyTick(stream_idx as u32 - 6));
                } else {
                    self.measure_buf.push(Muxml2TabElement::Rest(1));
                }
                trace!(kind = ?self.measure_buf.last().unwrap(), "Parsed a tick");
                (note_count, stream_proc_cnt) = (0, 0)
            } else {
                stream_proc_cnt += 1;
            }
        }
        if self.settings.remove_rest_between_notes {
            remove_rest_between_notes(&mut self.measure_buf, &mut measure_content_len);
        }
        merge_rests_in_measure(&mut self.measure_buf);
        if self.settings.trim_measure {
            trim_measure(&mut self.measure_buf, &mut measure_content_len, Direction::Forward);
            trim_measure(&mut self.measure_buf, &mut measure_content_len, Direction::Backward);
        }
        // Try to simplify e.g 8/8 to 4/4
        let (mut measure_enumerator, mut measure_denominator) = (measure_content_len, 8);
        if self.settings.simplify_time_signature && measure_content_len.is_multiple_of(2) {
            measure_enumerator /= 2;
            measure_denominator /= 2;
        }
        write_muxml2_measure_prelude(
            &mut self.document,
            measure_idx,
            measure_enumerator as usize,
            measure_denominator,
        )
        .unwrap();
        for i in 0..self.measure_buf.len() {
            self.write_tab_element(i)?;
        }
        self.document.push_str("</measure>");
        Ok(())
    }
    #[inline(always)]
    // takes an index because of borrowing schenanigans
    pub fn write_tab_element(&mut self, elem_idx: usize) -> std::fmt::Result {
        let elem = &self.measure_buf[elem_idx];
        match elem {
            Muxml2TabElement::Rest(x) => write_rest(&mut self.document, *x),
            Muxml2TabElement::CopyTick(tick_idx) => {
                let idu = *tick_idx as usize;
                let note_range = idu..=(idu + 5);
                let notes_iter = self.parsed.tick_stream[note_range.clone()]
                    .iter()
                    .enumerate()
                    .filter(|x| !matches!(x.1, TabElement::Rest))
                    .map(|(x, y)| (x + idu, y));
                // at least two notes here
                let tick_chord = notes_iter.clone().take(2).count() == 2;
                trace!(?note_range, chord = tick_chord, "for CopyTick({tick_idx})");
                // TODO: use dynamic base notes - we parse it but we don't use it
                for (elem_idx, elem) in notes_iter {
                    let Some((dead, fret)) = (match elem {
                        TabElement::DeadNote => Some((true, 0)),
                        TabElement::Fret(x) => Some((false, *x)),
                        _ => None,
                    }) else {
                        continue;
                    };
                    let string_name = self.parsed.base_notes[elem_idx % 6];
                    let note = get_fretboard_note2(string_name, fret).unwrap_or_else(|| {
                        panic!("Don't know base note for string name {string_name}",)
                    });
                    let (step, octave, sharp) = note.step_octave_sharp();
                    let properties = self.note_properties.get(&(elem_idx as u32));
                    let doc = &mut self.document;
                    let need_chord = tick_chord && elem_idx > 0;
                    write_muxml2_note(doc, step, octave, sharp, need_chord, dead, properties)?;
                }

                Ok(())
            }
            Muxml2TabElement::Invalid => Ok(()),
        }
    }
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
