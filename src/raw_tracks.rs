use crate::{
    backend::errors::{BackendError, BackendErrorKind, ErrorLocation},
    parser::{Measure, RawTick, Score, Section, TabElement},
};

pub type RawTracks = ([char; 6], [Vec<Measure>; 6]);
impl Score {
    pub fn gen_raw_tracks<'a>(self) -> Result<RawTracks, BackendError<'a>> {
        let diagnostics = vec![];
        let mut tracks = [vec![], vec![], vec![], vec![], vec![], vec![]];
        let mut track_names = ['\0'; 6];

        for part in self.0.into_iter() {
            match part {
                Section::Part { part, .. } => {
                    for (line_idx, line) in part.into_iter().enumerate() {
                        track_names[line_idx] = line.string_name;
                        for staff in line.measures {
                            tracks[line_idx].push(staff)
                        }
                    }
                }
                Section::Comment(_) => (),
            }
        }

        // This part is for correcting multichar frets (fret >=10)
        // because the parser above ^ will errorneously generate two rests
        // when there's a multichar fret on another string

        // we assume all tracks have equal measure count
        let measure_count = tracks[0].len();
        for measure_idx in 0..measure_count {
            let (mut tick_count, track_with_least_ticks) = tracks
                .iter()
                .enumerate()
                .map(|(track_idx, track)| (track[measure_idx].content.len(), track_idx))
                .min() // the string with the least ticks has the most twochar frets
                .expect("Empty score");
            //println!("[T]: tick count for measure {measure_idx}: {tick_count} (least on {track_with_least_ticks})");
            let mut tick_idx = 0;
            while tick_idx < tick_count {
                let Some((multichar_t_idx, RawTick { element: TabElement::Fret(multichar_fret), ..})) = tracks.iter().enumerate()
                    .map(|(track_idx, track)| {
        (track_idx,  track[measure_idx]
                            .content
                            .get(tick_idx)
                            .unwrap_or_else(|| panic!("Measure {measure_num} on string {string_name} doesn't have tick {tick_idx}\n", measure_num = measure_idx +1, string_name = track_names[track_idx] )))
                    })
                    .find(|(_,x)| { match x.element {
                        TabElement::Fret(x) => x >= 10,
                        _ => false,
                    }})
                else {
                    tick_idx += 1;
                    continue;
                };
                let multichar_fret = *multichar_fret;

                for track_idx in 0..tracks.len() {
                    let track = &mut tracks[track_idx];
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
                                let parent_line = measure.parent_line;
                                return Err(BackendError {
                                    main_location: ErrorLocation::LineAndCharIdx(
                                        parent_line,
                                        next.idx_on_parent_line,
                                    ),
                                    relevant_lines: parent_line..=parent_line,
                                    kind: BackendErrorKind::BadMulticharTick {
                                        multichar: (track_names[multichar_t_idx], multichar_fret),
                                        invalid: (track_names[track_idx], fret),
                                        tick_idx,
                                    },
                                    diagnostics,
                                });
                            }

                            // Beware: this is O(n)
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

        Ok((track_names, tracks))
    }
}

#[test]
fn test_multichar_raw_tracks() -> anyhow::Result<()> {
    use crate::parser::parser2::parse2;
    let input = r#"
e|----5--|
B|---3---|
G|10---12|
D|-------|
A|-------|
E|-------|"#;
    insta::assert_debug_snapshot!(parse2(input.lines()).unwrap().1.gen_raw_tracks());
    Ok(())
}
