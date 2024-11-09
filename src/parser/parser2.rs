use crate::{
    backend::errors::{
        backend_error::BackendError, backend_error_kind::BackendErrorKind, diagnostic::Diagnostic,
        diagnostic_kind::DiagnosticKind, error_location::ErrorLocation,
    },
    parser::{RawTick, TabElement},
    raw_tracks::find_multichar_tick,
};

use super::{comment_line, partline, Measure, Section};
#[derive(Debug)]
pub struct Parse2Result {
    pub diagnostics: Vec<Diagnostic>,
    pub sections: Vec<Section>,
    pub strings: [Vec<Measure>; 6],
    pub string_names: [char; 6],
    pub tick_cnt: usize,
}
pub fn parse2<'a, A: std::iter::Iterator<Item = &'a str>>(
    lines: A,
) -> Result<Parse2Result, BackendError<'a>> {
    let mut diagnostics = vec![];
    let mut sections = Vec::with_capacity(10);
    // Todo eventually remove Part
    let mut part_buf = Vec::with_capacity(6);
    let mut strings: [Vec<Measure>; 6] = [vec![], vec![], vec![], vec![], vec![], vec![]];
    let mut string_names = ['\0'; 6];
    let mut tick_cnt = 0;
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
            let measures_on_str = strings[part_buf.len()].len();

            match partline(
                line,
                line_idx,
                &mut strings[part_buf.len()],
                measures_on_str,
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
                        // flush part buf
                        sections.push(Section::Part {
                            part: part_buf
                                .try_into()
                                .expect("Unreachable: more than 6 elements in part_buf"),
                        });
                        part_buf = Vec::with_capacity(6);
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

    // This part is for correcting multichar frets (fret >=10)
    // because the parser above ^ will errorneously generate two rests
    // when there's a multichar fret on another string

    // we assume all tracks have equal measure count
    let measure_count = strings[0].len();
    for measure_idx in 0..measure_count {
        let (mut tick_count, track_with_least_ticks) = strings
            .iter()
            .enumerate()
            .map(|(track_idx, track)| (track[measure_idx].content.len(), track_idx))
            .min() // the string with the least ticks has the most twochar frets
            .expect("Empty score");
        //println!("[T]: tick count for measure {measure_idx}: {tick_count} (least on {track_with_least_ticks})");
        let mut tick_idx = 0;
        while tick_idx < tick_count {
            let Some((
                multichar_t_idx,
                RawTick {
                    element: TabElement::Fret(multichar_fret),
                    ..
                },
            )) = find_multichar_tick(&strings, measure_idx, string_names, tick_idx)
            else {
                tick_idx += 1;
                continue;
            };
            let multichar_fret = *multichar_fret;

            for track_idx in 0..strings.len() {
                let track = &mut strings[track_idx];
                let measure = &mut track[measure_idx];
                // This is a multi-char tick. Remove adjacent rest everywhere where it is not
                // multi-char.
                let tick_onechar_on_this_track = match &measure.content[tick_idx].element {
                    TabElement::Fret(x) => *x < 10,
                    TabElement::Rest => true,
                    TabElement::DeadNote => true,
                };
                if tick_onechar_on_this_track {
                    if let Some(next) = measure.content.get(tick_idx + 1) {
                        if let TabElement::Fret(fret) = next.element {
                            #[rustfmt::skip]
                                return Err(BackendError::bad_multichar_tick(diagnostics, measure.parent_line, next.idx_on_parent_line, string_names[multichar_t_idx], multichar_fret, string_names[track_idx], fret, tick_idx));
                        }

                        // Beware: this is O(n). I don't think this can be done in a better way, and measures are typically not that big.
                        measure.content.remove(tick_idx + 1);
                        if track_idx == track_with_least_ticks {
                            tick_count -= 1;
                        }
                    }
                }
            }
            tick_idx += 1;
        }
    }

    Ok(Parse2Result {
        diagnostics,
        sections,
        strings,
        string_names,
        tick_cnt,
    })
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
}
