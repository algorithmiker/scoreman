use std::{collections::HashMap, time::Instant};

use midly::{
    num::{u28, u7},
    Format, Header, MetaMessage, MidiMessage, Smf, TrackEvent, TrackEventKind,
};

use crate::{
    parser::{
        parser2::{Parse2Result, Parser2, ParserInput},
        TabElement::*,
    },
    time,
};

use super::{Backend, BackendResult};

const BPM: u32 = 80;
const MINUTE_IN_MICROSECONDS: u32 = 60 * 1000;
const LENGTH_OF_QUARTER: u32 = MINUTE_IN_MICROSECONDS / BPM;
const LENGTH_OF_EIGHT: u32 = LENGTH_OF_QUARTER / 2;

pub struct MidiBackend();
impl Backend for MidiBackend {
    type BackendSettings = ();

    fn process<'a, Out: std::io::Write>(
        input: impl ParserInput<'a>,
        out: &mut Out,
        _settings: Self::BackendSettings,
    ) -> BackendResult<'a> {
        let mut diagnostics = vec![];
        let parser = Parser2 {
            track_measures: false,
            track_sections: false,
        };
        let (parse_time, parse_result) = match time(|| parser.parse(input)) {
            (parse_time, Ok(parse_result)) => (parse_time, parse_result),
            (_, Err(err)) => return BackendResult::new(diagnostics, Some(err), None, None),
        };
        // TODO: the parser now gives us things like tick count, can probably preallocate based on
        // that
        let gen_start = Instant::now();
        let mut midi_tracks = convert_to_midi(&parse_result);
        diagnostics.extend(parse_result.diagnostics);
        let mut tracks = vec![vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 4, 1, 8)),
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(LENGTH_OF_QUARTER.into())),
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            },
        ]];
        tracks.append(&mut midi_tracks);
        let smf = Smf {
            header: Header::new(Format::Parallel, midly::Timing::Metrical(4.into())),
            tracks,
        };
        let gen_time = gen_start.elapsed();
        if let Err(x) = smf.write_std(out) {
            return BackendResult::new(
                diagnostics,
                Some(x.into()),
                Some(parse_time),
                Some(gen_time),
            );
        }
        BackendResult::new(diagnostics, None, Some(parse_time), Some(gen_time))
    }
}

fn convert_to_midi(parse_result: &Parse2Result) -> Vec<Vec<TrackEvent<'static>>> {
    let mut string_freq = HashMap::new();
    string_freq.insert('E', 52);
    string_freq.insert('A', 57);
    string_freq.insert('D', 62);
    string_freq.insert('G', 67);
    string_freq.insert('B', 71);
    string_freq.insert('d', 74);
    string_freq.insert('e', 76);
    let mut tracks: Vec<Vec<TrackEvent>> = vec![Vec::new(); 6];

    #[allow(clippy::needless_range_loop)]
    for i in 0..6 {
        let string_name = parse_result.string_names[i];
        let raw_track = &parse_result.strings[i];
        let mut delta_carry: u32 = 0;
        for (tick_idx, raw_tick) in raw_track.iter().enumerate() {
            match &raw_tick.element {
                Fret(fret) => {
                    let pitch = fret + string_freq[&string_name];
                    let (note_on, note_off) = gen_note_events(pitch.into(), delta_carry.into());
                    delta_carry = 0;
                    tracks[i].push(note_on);
                    tracks[i].push(note_off);
                }
                FretBend(fret) => {
                    let pitch = fret + string_freq[&string_name] + 1;
                    let (note_on, note_off) = gen_note_events(pitch.into(), delta_carry.into());
                    delta_carry = 0;
                    tracks[i].push(note_on);
                    tracks[i].push(note_off);

                    let pitch = pitch + 1;
                    let (note_on, note_off) = gen_note_events(pitch.into(), delta_carry.into());
                    delta_carry = 0;
                    tracks[i].push(note_on);
                    tracks[i].push(note_off);
                }
                FretBendTo(from) => {
                    let pitch = from + string_freq[&string_name];
                    let (note_on, note_off) = gen_note_events(pitch.into(), delta_carry.into());
                    delta_carry = 0;
                    tracks[i].push(note_on);
                    tracks[i].push(note_off);

                    let to = parse_result
                        .bend_targets
                        .get(&(i as u8, tick_idx as u32))
                        .expect("Unreachable: FretBendTo without target");
                    let pitch = to + string_freq[&string_name];
                    let (note_on, note_off) = gen_note_events(pitch.into(), delta_carry.into());
                    delta_carry = 0;
                    tracks[i].push(note_on);
                    tracks[i].push(note_off);
                }
                Rest => delta_carry += LENGTH_OF_EIGHT,
                DeadNote => (),
            }
        }
        tracks[i].push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
    }
    tracks
}

fn gen_note_events<'a>(key: u7, initial_delta: u28) -> (TrackEvent<'a>, TrackEvent<'a>) {
    let note_on = TrackEvent {
        delta: initial_delta,
        kind: TrackEventKind::Midi {
            channel: 0.into(),
            message: MidiMessage::NoteOn {
                key,
                vel: 100.into(),
            },
        },
    };

    let note_off = TrackEvent {
        delta: LENGTH_OF_EIGHT.into(),
        kind: TrackEventKind::Midi {
            channel: 0.into(),
            message: MidiMessage::NoteOff {
                key,
                vel: 100.into(),
            },
        },
    };
    (note_on, note_off)
}
