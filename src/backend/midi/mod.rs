use std::{collections::HashMap, iter, time::Instant};

use midly::{
    num::{u28, u7},
    Format, Header, MetaMessage, MidiMessage, Smf, TrackEvent, TrackEventKind,
};
use tracing::trace;

use super::{Backend, BackendResult};
use crate::{parser::ParserResult, time};
use crate::{
    parser::{
        tab_element::TabElement::{self, Fret},
        Parser,
    },
    BufLines,
};

const BPM: u32 = 80;
const MINUTE_IN_MS: u32 = 60 * 1000;
const MINUTE_IN_US: u32 = MINUTE_IN_MS * 1000;
const LENGTH_OF_QUARTER: u32 = MINUTE_IN_US / BPM;
const LENGTH_OF_EIGHTH: u32 = 1;

pub struct MidiBackend();
impl Backend for MidiBackend {
    type BackendSettings = ();

    fn process<Out: std::io::Write>(
        input: &BufLines, out: &mut Out, _settings: Self::BackendSettings,
    ) -> BackendResult {
        let diagnostics = vec![];
        let (parse_time, parsed0) = time(|| Parser::parse(input));
        let parsed = match parsed0 {
            Ok(x) => x,
            Err(y) => return BackendResult::new(diagnostics, Some(y.0), Some(parse_time), None),
        };
        // TODO: the parser now gives us things like tick count, can probably preallocate based on
        // that
        let gen_start = Instant::now();
        let mut midi_tracks = convert_to_midi(&parsed);
        //diagnostics.extend(parse_result.diagnostics);
        trace!(LENGTH_OF_QUARTER, "Length of quarter");
        let mut tracks = vec![vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 4, 24, 8)),
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(LENGTH_OF_QUARTER.into())),
            },
            TrackEvent { delta: 0.into(), kind: TrackEventKind::Meta(MetaMessage::EndOfTrack) },
        ]];
        tracks.append(&mut midi_tracks);
        let smf = Smf {
            header: Header::new(Format::Parallel, midly::Timing::Metrical(4.into())),
            tracks,
        };
        let gen_time = gen_start.elapsed();
        let mut last_err = None;
        if let Err(x) = smf.write_std(out) {
            last_err = Some(x.into());
        };
        BackendResult::new(diagnostics, last_err, Some(parse_time), Some(gen_time))
    }
}

fn convert_to_midi(parsed: &ParserResult) -> Vec<Vec<TrackEvent<'static>>> {
    // TODO: maybe use the traditional note resolving logic here?
    let mut string_freq = HashMap::new();
    string_freq.insert('E', 52);
    string_freq.insert('A', 57);
    string_freq.insert('D', 62);
    string_freq.insert('G', 67);
    string_freq.insert('B', 71);
    string_freq.insert('d', 74);
    string_freq.insert('e', 76);
    let track_len = parsed.tick_stream.len() / 6;
    // https://rust-lang.github.io/rust-clippy/master/index.html#repeat_vec_with_capacity
    let mut tracks: Vec<Vec<TrackEvent>> =
        iter::repeat_with(|| Vec::with_capacity(track_len)).take(6).collect();
    let mut delta_carry_on = [u28::new(0); 6];
    for (event_idx, event) in parsed.tick_stream.iter().enumerate() {
        // TODO: eventually try to interpolate for slurred decorators
        let track = event_idx % 6;
        match &event {
            Fret(fret) => {
                let string_name = parsed.base_notes[track];
                let pitch = fret + string_freq[&string_name];
                let (note_on, note_off) = gen_note_events(pitch.into(), delta_carry_on[track]);
                delta_carry_on[track] = 0.into();
                tracks[track].push(note_on);
                tracks[track].push(note_off);
            }
            TabElement::Rest => delta_carry_on[track] += LENGTH_OF_EIGHTH.into(),
            TabElement::Bend
            | TabElement::HammerOn
            | TabElement::Pull
            | TabElement::Release
            | TabElement::Slide
            | TabElement::DeadNote
            | TabElement::Vibrato => (),
        }
    }
    tracks.iter_mut().for_each(|x| {
        x.push(TrackEvent { delta: 0.into(), kind: TrackEventKind::Meta(MetaMessage::EndOfTrack) })
    });
    tracks
}

fn gen_note_events<'a>(key: u7, initial_delta: u28) -> (TrackEvent<'a>, TrackEvent<'a>) {
    let note_on = TrackEvent {
        delta: initial_delta,
        kind: TrackEventKind::Midi {
            channel: 0.into(),
            message: MidiMessage::NoteOn { key, vel: 100.into() },
        },
    };

    let note_off = TrackEvent {
        delta: LENGTH_OF_EIGHTH.into(),
        kind: TrackEventKind::Midi {
            channel: 0.into(),
            message: MidiMessage::NoteOff { key, vel: 100.into() },
        },
    };
    (note_on, note_off)
}
