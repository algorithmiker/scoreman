use crate::{
    backend::errors::{
        backend_error::BackendError, backend_error_kind::BackendErrorKind, diagnostic::Diagnostic,
        diagnostic_kind::DiagnosticKind, error_location::ErrorLocation,
    },
    parser::{RawTick, TabElement},
};

use super::{comment_line, partline, Measure, Partline, Section};
#[derive(Debug)]
pub struct Parse2Result {
    pub diagnostics: Vec<Diagnostic>,
    pub sections: Vec<Section>,

    /// How many ticks there are in the document (sum of the line-tick-counts)
    pub tick_cnt: usize,
    pub string_names: [char; 6],
    pub strings: [Vec<RawTick>; 6],
    pub measures: [Vec<Measure>; 6],
}
pub fn parse2<'a, A: std::iter::Iterator<Item = &'a str>>(
    lines: A,
) -> Result<Parse2Result, BackendError<'a>> {
    let mut diagnostics = vec![];
    let mut sections = Vec::with_capacity(10);
    // Todo eventually remove Part
    let mut part_buf = Vec::with_capacity(6);
    let mut part_start_tick = 0;
    let mut strings: [Vec<RawTick>; 6] = [vec![], vec![], vec![], vec![], vec![], vec![]];
    let mut string_measures: [Vec<Measure>; 6] = [vec![], vec![], vec![], vec![], vec![], vec![]];
    let mut string_names = ['\0'; 6];
    let mut tick_cnt = 0;
    //    let mut idx_in_src = 0;
    for (line_idx, line) in lines.enumerate() {
        if line.trim().is_empty() {
            if !part_buf.is_empty() {
                diagnostics.push(Diagnostic::warn(
                    ErrorLocation::LineOnly(line_idx),
                    DiagnosticKind::EmptyLineInPart,
                ));
            }
            continue;
        }
        // +1 for \n
        //idx_in_src += line.len() + 1;

        if let Ok((rem, comment)) = comment_line(line) {
            // I don't think there is a way to write an invalid comment after a valid start, just to be safe
            assert!(rem.is_empty(), "Invalid comment syntax (line {line_idx})");
            if !part_buf.is_empty() {
                diagnostics.push(Diagnostic::warn(
                    ErrorLocation::LineOnly(line_idx),
                    DiagnosticKind::CommentInPart,
                ));
            }
            sections.push(Section::Comment(comment.to_string()));
        } else {
            match partline(
                line,
                line_idx,
                &mut strings[part_buf.len()],
                &mut string_measures[part_buf.len()],
            ) {
                Ok((rem, (line, l_tick_count))) => {
                    if !rem.is_empty() {
                        return Err(BackendError {
                            main_location: ErrorLocation::LineAndMeasure(
                                line_idx,
                                // the measure with the problem is the first that is not parsed
                                line.len(),
                            ),
                            relevant_lines: line_idx..=line_idx,
                            kind: BackendErrorKind::InvalidPartlineSyntax(rem),
                            diagnostics,
                        });
                    }
                    tick_cnt += l_tick_count;
                    string_names[part_buf.len()] = line.string_name;
                    part_buf.push(line);
                    if part_buf.len() == 6 {
                        // This part is for correcting multichar frets (fret >=10)
                        // because the parser will errorneously generate two rests
                        // when there's a multichar fret on another string
                        if let Err((kind, char)) = fixup_part(
                            part_start_tick,
                            &mut part_buf,
                            &mut strings,
                            &mut string_measures,
                            &string_names,
                        ) {
                            return Err(BackendError {
                                main_location: ErrorLocation::LineAndCharIdx(line_idx, char),
                                relevant_lines: line_idx..=line_idx,
                                kind,
                                diagnostics,
                            });
                        }
                        // flush part buf
                        sections.push(Section::Part {
                            part: part_buf
                                .try_into()
                                .expect("Unreachable: more than 6 elements in part_buf"),
                        });
                        part_buf = Vec::with_capacity(6);
                        part_start_tick = strings[0].len();
                    }
                }
                Err(x) => {
                    return Err(BackendError {
                        main_location: ErrorLocation::LineOnly(line_idx),
                        relevant_lines: line_idx..=line_idx,
                        kind: BackendErrorKind::ParseError(x),
                        diagnostics,
                    });
                }
            }
        }
    }

    // TODO: do this *while constructing the track somehow, for optimal performance
    Ok(Parse2Result {
        diagnostics,
        sections,
        measures: string_measures,
        strings,
        string_names,
        tick_cnt,
    })
}
fn fixup_part(
    // we only need to check after this
    start_tick: usize,
    part: &mut [Partline],
    strings: &mut [Vec<RawTick>; 6],
    measures: &mut [Vec<Measure>; 6],
    string_names: &[char; 6],
) -> Result<(), (BackendErrorKind<'static>, usize)> {
    let (mut tick_count, track_with_least_ticks) = strings
        .iter()
        .enumerate()
        .map(|(track_idx, track)| (track.len(), track_idx))
        .min() // the string with the least ticks has the most twochar frets
        .expect("Empty score");
    let mut tick_idx = start_tick;
    while tick_idx < tick_count {
        let Some((
            multichar_t_idx,
            RawTick {
                element: TabElement::Fret(multichar_fret),
                ..
            },
        )) = ({
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
                .find(|(_, x)| match x.element {
                    TabElement::Fret(x) => x >= 10,
                    _ => false,
                })
        })
        else {
            tick_idx += 1;
            continue;
        };
        // so we stop borrowing strings
        let multichar_fret = *multichar_fret;
        // This is a multi-char tick. Remove adjacent rest everywhere where it is not
        // multi-char.
        for string_idx in 0..6 {
            let tick_onechar_on_this_track = match strings[string_idx][tick_idx].element {
                TabElement::Fret(x) => x < 10,
                TabElement::Rest | TabElement::DeadNote => true,
            };
            if tick_onechar_on_this_track {
                if let Some(next) = strings[string_idx].get(tick_idx + 1) {
                    if let TabElement::Rest = next.element {
                        // remove the next tick
                        // TODO: O(N) but should be few elements after
                        strings[string_idx].remove(tick_idx + 1);
                        // now also update measure information to stay correct
                        for measure_idx in part[string_idx].measures.clone() {
                            let mc = &mut measures[string_idx][measure_idx].content;
                            if *mc.start() > tick_idx {
                                // move measure to the right
                                *mc = mc.start() - 1..=mc.end() - 1;
                            } else if *mc.end() > tick_idx {
                                // pop one from end
                                *mc = *mc.start()..=mc.end() - 1
                            }
                        }
                        if string_idx == track_with_least_ticks {
                            tick_count -= 1;
                        }
                    } else {
                        return Err((
                            BackendErrorKind::BadMulticharTick {
                                multichar: (string_names[multichar_t_idx], multichar_fret),
                                invalid: (string_names[string_idx], next.element.clone()),
                                tick_idx,
                            },
                            next.idx_on_parent_line,
                        ));
                    }
                }
            }
        }
        tick_idx += 1;
    }
    Ok(())
    //let measure_count = strings[0].len();
    //for measure_idx in part[0].measures {
    //    let (mut tick_count, track_with_least_ticks) = strings
    //        .iter()
    //        .enumerate()
    //        .map(|(track_idx, track)| (measures[track_idx][measure_idx].len(), track_idx))
    //        .min() // the string with the least ticks has the most twochar frets
    //        .expect("Empty score");
    //    //println!("[T]: tick count for measure {measure_idx}: {tick_count} (least on {track_with_least_ticks})");
    //    let mut tick_idx = 0;
    //    while tick_idx < tick_count {
    //        let Some((
    //            multichar_t_idx,
    //            RawTick {
    //                element: TabElement::Fret(multichar_fret),
    //                ..
    //            },
    //        )) = find_multichar_tick(&strings, &measures, measure_idx, string_names, tick_idx)
    //        else {
    //            tick_idx += 1;
    //            continue;
    //        };
    //        let multichar_fret = *multichar_fret;

    //        for track_idx in 0..strings.len() {
    //            let track = &mut strings[track_idx];
    //            // This is a multi-char tick. Remove adjacent rest everywhere where it is not
    //            // multi-char.
    //            let tick_onechar_on_this_track = match measures[track_idx][measure_idx]
    //                .get_content(&strings[track_idx])[tick_idx]
    //                .element
    //            {
    //                TabElement::Fret(x) => x < 10,
    //                TabElement::Rest => true,
    //                TabElement::DeadNote => true,
    //            };
    //            if tick_onechar_on_this_track {
    //                if let Some(next) = measures[track_idx][measure_idx]
    //                    .get_content(&strings[track_idx])
    //                    .get(tick_idx + 1)
    //                {
    //                    if let TabElement::Fret(fret) = next.element {
    //                        #[rustfmt::skip]
    //                            return Err(BackendError::bad_multichar_tick(diagnostics, measure.parent_line, next.idx_on_parent_line, string_names[multichar_t_idx], multichar_fret, string_names[track_idx], fret, tick_idx));
    //                    }

    //                    // remove this tick
    //                    measures[track_idx][measure_idx].pop_1();
    //                    // HACK: O(N)
    //                    strings[track_idx].
    //                    if track_idx == track_with_least_ticks {
    //                        tick_count -= 1;
    //                    }
    //                }
    //            }
    //        }
    //        tick_idx += 1;
    //    }
    //}
}
#[test]
fn test_parse2() {
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
    insta::assert_debug_snapshot!(parse2(i1.lines()));
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
    insta::assert_debug_snapshot!(parse2(i2.lines()));

    let i3 = r#"
e|-------------12---------------------|
B|-------------3---0--------------3---|
G|---------0-2-------2-0--------------|
D|---0-2-3---------------3-2-0--------|
A|-3---------------------------3------|
E|------------------------------------|"#;
    insta::assert_debug_snapshot!(parse2(i3.lines()));
}
