use std::collections::HashMap;

use crate::backend::errors::error_location::SourceOffset;
use crate::{
    backend::errors::{
        backend_error::BackendError, backend_error_kind::BackendErrorKind, diagnostic::Diagnostic,
        diagnostic_kind::DiagnosticKind, error_location::ErrorLocation,
    },
    parser::{RawTick, TabElement},
};

use super::{comment_line, partline, Measure, Partline, Section};
pub type BendTargets = HashMap<(u8, u32), u8>;
#[derive(Debug)]
pub struct Parse2Result {
    pub diagnostics: Vec<Diagnostic>,
    pub sections: Vec<Section>,

    pub string_names: [char; 6],
    pub strings: [Vec<RawTick>; 6],
    pub measures: [Vec<Measure>; 6],
    pub offsets: [Vec<u32>; 6],
    pub bend_targets: BendTargets,
}

// TODO: try if using bitflags would speed this up
pub struct Parser2 {
    pub track_measures: bool,
    pub track_sections: bool,
}
impl Default for Parser2 {
    fn default() -> Self {
        Self {
            track_measures: true,
            track_sections: true,
        }
    }
}
pub trait ParserInput<'a>: std::iter::Iterator<Item = &'a str> {}
impl<'a, T: std::iter::Iterator<Item = &'a str>> ParserInput<'a> for T {}
impl Parser2 {
    // TODO: add a way to discard measure/part information for backends that don't need it
    // Will probably involve a restructuring of the parsing step to be controlled by the backend.
    // I imagine a Parser {settings: ParserSettings }.parse()
    pub fn parse<'a>(&self, lines: impl ParserInput<'a>) -> Result<Parse2Result, BackendError<'a>> {
        let mut diagnostics = vec![];
        #[rustfmt::skip]
        let mut sections = if self.track_sections{Vec::with_capacity(10)} else {vec![]};
        let mut part_buf = Vec::with_capacity(6);
        let mut line_in_part = 0;
        let mut part_start_tick = 0;
        let mut strings: [Vec<RawTick>; 6] = [const { Vec::new() }; 6];
        let mut string_measures: [Vec<Measure>; 6] = [const { Vec::new() }; 6];
        let mut offsets: [Vec<u32>; 6] = [const { Vec::new() }; 6];
        let mut string_names = ['\0'; 6];
        let mut source_offset = 0u32;
        // TODO: try using integer hashing
        let mut bend_targets = HashMap::new();
        for (line_idx, line) in lines.enumerate() {
            if line.trim().is_empty() {
                if line_in_part != 0 {
                    diagnostics.push(Diagnostic::warn(
                        ErrorLocation::LineOnly(line_idx),
                        DiagnosticKind::EmptyLineInPart,
                    ));
                }
                source_offset += line.len() as u32 + 1;
                continue;
            }

            if let Ok((rem, comment)) = comment_line(line) {
                // I don't think there is a way to write an invalid comment after a valid start, just to be safe
                assert!(rem.is_empty(), "Invalid comment syntax (line {line_idx})");
                if line_in_part != 0 {
                    diagnostics.push(Diagnostic::warn(
                        ErrorLocation::LineOnly(line_idx),
                        DiagnosticKind::CommentInPart,
                    ));
                }
                if self.track_sections {
                    sections.push(Section::Comment(comment.to_string()))
                };
            } else {
                match partline(
                    line,
                    line_idx,
                    source_offset,
                    &mut strings[line_in_part],
                    &mut string_measures[line_in_part],
                    &mut offsets[line_in_part],
                    &mut bend_targets,
                    line_in_part,
                    self.track_measures,
                ) {
                    Ok((rem, line)) => {
                        if !rem.is_empty() {
                            return Err(BackendError {
                                // the measure with the problem is the first that is not parsed
                                main_location: ErrorLocation::LineAndMeasure(line_idx, line.len()),
                                relevant_lines: line_idx..=line_idx,
                                kind: BackendErrorKind::InvalidPartlineSyntax(rem),
                            });
                        }

                        string_names[line_in_part] = line.string_name;
                        part_buf.push(line);
                        line_in_part += 1;
                        if line_in_part == 6 {
                            // This part is for correcting multichar frets (fret >=10)
                            // because the parser will errorneously generate two rests
                            // when there's a multichar fret on another string
                            if let Err((kind, invalid_offset, invalid_line_idx)) = fixup_part(
                                part_start_tick,
                                &mut part_buf,
                                &mut strings,
                                &mut string_measures,
                                &mut offsets,
                                &string_names,
                                &bend_targets,
                                self.track_measures,
                            ) {
                                return Err(BackendError {
                                    main_location: ErrorLocation::SourceOffset(SourceOffset::new(
                                        invalid_offset,
                                    )),
                                    relevant_lines: invalid_line_idx..=invalid_line_idx,
                                    kind,
                                });
                            }
                            if self.track_sections {
                                // flush part buf
                                sections.push(Section::Part {
                                    part: part_buf.try_into().unwrap(),
                                });
                            }
                            part_buf = Vec::with_capacity(6);
                            line_in_part = 0;
                            part_start_tick = strings[0].len();
                        }
                    }
                    Err(x) => {
                        return Err(BackendError {
                            main_location: ErrorLocation::LineOnly(line_idx),
                            relevant_lines: line_idx..=line_idx,
                            kind: BackendErrorKind::InvalidPartlineSyntax(x),
                        });
                    }
                }
            }

            // +1 for \n
            source_offset += line.len() as u32 + 1
        }
        Ok(Parse2Result {
            diagnostics,
            sections,
            measures: string_measures,
            strings,
            string_names,
            offsets,
            bend_targets,
        })
    }
}

fn fixup_part(
    // we only need to check after this
    start_tick: usize,
    part: &mut [Partline],
    strings: &mut [Vec<RawTick>; 6],
    measures: &mut [Vec<Measure>; 6],
    offsets: &mut [Vec<u32>; 6],
    string_names: &[char; 6],
    bend_targets: &BendTargets,
    track_measures: bool,
) -> Result<(), (BackendErrorKind<'static>, usize, usize)> {
    //println!("initial view of fixup_parts:\n{}", dump_tracks(strings));
    // TODO: i think we can early exit here if we have the same length on all strings, not sure tho
    let (mut tick_count, track_with_least_ticks) = strings
        .iter()
        .enumerate()
        .map(|(track_idx, track)| (track.len(), track_idx))
        .min() // the string with the least ticks has the most multichar frets
        .expect("Empty score");
    let mut tick_idx = start_tick;
    while tick_idx < tick_count {
        //println!("tick_idx={tick_idx}");
        let Some((multichar_track, multichar_len, RawTick { .. })) = ({
            strings
                .iter()
                .enumerate()
                .map(|(t_idx, track)| {
                    (
                        t_idx,
                        track.get(tick_idx).unwrap_or_else(|| {
                            panic!(
                                "String {} doesn't have tick {tick_idx}\n",
                                string_names[t_idx]
                            );
                        }),
                    )
                })
                .map(|(t_idx, x)| {
                    (
                        t_idx,
                        x.element
                            .repr_len(bend_targets, &(t_idx as u8, tick_idx as u32)),
                        x,
                    )
                })
                .find(|(_, len, _)| *len > 1)
        }) else {
            tick_idx += 1;
            continue;
        };
        //println!("  this is a multi-char tick");
        // This is a multi-char tick. Remove adjacent rest everywhere where it is not
        // multi-char.
        for string_idx in 0..6 {
            let chars_here = strings[string_idx][tick_idx]
                .element
                .repr_len(bend_targets, &(string_idx as u8, tick_idx as u32));
            //println!(
            //    "  string {string_idx}, chars here: {chars_here} (elem: {:?})",
            //    strings[string_idx][tick_idx]
            //);
            fn try_remove_from_right(
                string: &mut Vec<RawTick>,
                offsets: &mut Vec<u32>,
                tick_idx: usize,
                count: usize,
            ) -> bool {
                let first_rest = string[tick_idx].element == TabElement::Rest;
                let shared_range_good = tick_idx + count < string.len() + 1
                    && string[tick_idx + 1..tick_idx + count]
                        .iter()
                        .all(|x| x.element == TabElement::Rest);
                if first_rest && shared_range_good {
                    string.drain(tick_idx..tick_idx + count).for_each(drop);
                    offsets.drain(tick_idx..tick_idx + count).for_each(drop);
                    return true;
                } else if shared_range_good
                    && tick_idx + count < string.len()
                    && string[tick_idx + count].element == TabElement::Rest
                {
                    string
                        .drain(tick_idx + 1..tick_idx + count + 1)
                        .for_each(drop);
                    offsets.drain(tick_idx..tick_idx + count).for_each(drop);
                    return true;
                }
                false
            }

            if chars_here < multichar_len {
                let cnt_to_remove = (multichar_len - chars_here) as usize;

                if !try_remove_from_right(
                    &mut strings[string_idx],
                    &mut offsets[string_idx],
                    tick_idx,
                    (multichar_len - chars_here) as usize,
                ) {
                    // TODO: make the internal track representation part of the error
                    // println!("  view before hitting error:");
                    // println!("{}", dump_tracks(strings));
                    return Err((
                        BackendErrorKind::BadMulticharTick {
                            multichar: (
                                string_names[multichar_track],
                                strings[multichar_track][tick_idx].element.clone(),
                            ),
                            invalid: (
                                string_names[string_idx],
                                strings[string_idx][tick_idx].element.clone(),
                            ),
                            tick_idx: tick_idx as u32,
                        },
                        offsets[string_idx][tick_idx + 1] as usize,
                        string_idx,
                    ));
                }
                if track_measures {
                    // now also update measure information to stay correct
                    for measure_idx in part[string_idx].measures.clone() {
                        let mc = &mut measures[string_idx][measure_idx].content;
                        if *mc.start() > tick_idx {
                            // move measure to the right
                            *mc = mc.start() - cnt_to_remove..=mc.end() - cnt_to_remove;
                        } else if *mc.end() > tick_idx {
                            // pop one from end
                            *mc = *mc.start()..=mc.end() - cnt_to_remove
                        }
                    }
                }
                if string_idx == track_with_least_ticks {
                    tick_count -= cnt_to_remove;
                }
            }
        }
        tick_idx += 1;
    }
    //println!("after fixup:\n{}", dump_tracks(strings));
    Ok(())
}
#[test]
fn test_parse2() {
    let parser = Parser2::default();
    let i1 = r#"
e|---|
B|-3-|
G|6-6|
D|---|
A|---|
E|---|

// This is a comment

e|---|
B|3-3|
G|-6-|
D|---|
A|---|
E|---|"#;
    insta::assert_debug_snapshot!(parser.parse(i1.lines()));
    let i2 = r#"
e|---|
B|-3-|
G|6-6|
D|---|
A|-o-|
E|---|

// This is a comment

e|---|
B|3-3|
G|-6-|
D|---|
A|---|
E|---|"#;
    insta::assert_debug_snapshot!(parser.parse(i2.lines()));

    let i3 = r#"
e|-------------12---------------------|
B|-------------3---0--------------3---|
G|---------0-2-------2-0--------------|
D|---0-2-3---------------3-2-0--------|
A|-3---------------------------3------|
E|------------------------------------|"#;
    insta::assert_debug_snapshot!(parser.parse(i3.lines()));
    let i3 = r#"
e|-------------12---------------------|
B|-------------3---0--------------3---|
G|---------0-2-------2-0--------------|
D|---0-2-3---------------3-2-0--------|
A|-3---------------------------3------|
E|0-----------------------------------|"#;
    insta::assert_debug_snapshot!(parser.parse(i3.lines()));
}
