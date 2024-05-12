use anyhow::{bail, Context};

use crate::parser::{Measure, Score, Section, TabElement};

pub type RawTracks = ([char; 6], [Vec<Measure>; 6]);
impl Score {
    pub fn gen_raw_tracks(self) -> anyhow::Result<RawTracks> {
        let mut tracks = [vec![], vec![], vec![], vec![], vec![], vec![]];
        let mut track_names = ['\0'; 6];
        for part in self.0.into_iter() {
            match part {
                Section::Part { part, .. } => {
                    for (line_idx, line) in part.into_iter().enumerate() {
                        track_names[line_idx] = line.string_name;
                        for staff in line.staffs {
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
                .with_context(|| "Empty score")?;
            //println!("[T]: tick count for measure {measure_idx}: {tick_count} (least on {track_with_least_ticks})");
            let mut tick_idx = 0;
            while tick_idx < tick_count {
                let tick_has_multichar = tracks
                    .iter().enumerate()
                    .map(|(track_idx, track)| {
                        track[measure_idx]
                            .content
                            .get(tick_idx)
                            .unwrap_or_else(|| panic!("Measure {measure_num} on string {string_name} doesn't have tick {tick_idx}\n[I] Tip: use the format backend to check what's wrong", measure_num = measure_idx +1, string_name = track_names[track_idx] ))
                    })
                    .any(|x| { match x {
                        TabElement::Fret(x) => *x >= 10,
                        _ => false,
                    }});
                if !tick_has_multichar {
                    tick_idx += 1;
                    continue;
                }

                for (track_idx, track) in tracks.iter_mut().enumerate() {
                    let measure = &mut track[measure_idx];
                    // This is a multi-char tick. Remove adjacent rest everywhere where it is not
                    // multi-char.
                    let should_remove_rest = match &measure.content[tick_idx] {
                        TabElement::Fret(x) => *x < 10,
                        TabElement::Rest => true,
                    };
                    if should_remove_rest {
                        if let Some(next) = measure.content.get(tick_idx + 1) {
                            if *next != TabElement::Rest {
                                bail!(
                                    "Invalid multichar tick
Where: Line {line}, measure {measure_num}, multichar tick from {tick_num} to {next_tick_num}
Tick {tick_idx} has a multi-char (fret>=10) fret on some string above, but on the same tick there is an invalid {next:?} on string {string_name}",
                                    next_tick_num = tick_idx + 2,
                                    tick_num = tick_idx+1,
                                    line = measure.parent_line.unwrap() + 1,
                                    string_name = track_names[track_idx],
                                    measure_num = measure.index_on_parent_line.unwrap() + 1
                                );
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
    use std::io::BufReader;
    let input = r#"
e|----5--|
B|---3---|
G|10---12|
D|-------|
A|-------|
E|-------|"#;
    insta::assert_debug_snapshot!(parse2(BufReader::new(input.as_bytes()))?.gen_raw_tracks());
    Ok(())
}
