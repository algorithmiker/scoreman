#![allow(dead_code, unused_variables)]
use ndarray::{arr2, Array2};
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::time::Instant;

use super::{comment_line, dump_tracks, string_name, tab_element, Measure, RawTick, TabElement};
use crate::parser::parser2::Parser2;
use crate::{
    backend::{errors::backend_error::BackendError, muxml2::Muxml2TabElement},
    traceln,
};

pub fn line_is_valid(line: &str) -> bool {
    let line = line.trim();
    let first_is_alphanumeric = line
        .chars()
        .next()
        .map(|x| x.is_alphanumeric())
        .unwrap_or(false);
    let last_is_measure_end = line.as_bytes().last().map(|x| *x == b'|').unwrap_or(false);
    let ret = first_is_alphanumeric && last_is_measure_end;
    traceln!("line_is_valid({line}) -> {ret}");
    ret
}
struct Measure3 {
    pub data_range: RangeInclusive<u32>,
}
struct MeasureState {}
pub fn parse3(lines: &[String]) {
    let mut part_first_line = 0;
    let mut ticks = vec![]; // a stream of 6 tabelements after each other
    let mut base_notes = vec![]; // a stream of base notes by part
    let mut measures: Vec<Measure3> = vec![];
    while part_first_line + 5 < lines.len() {
        // find a part
        while part_first_line + 5 < lines.len() {
            if line_is_valid(&lines[part_first_line]) && line_is_valid(&lines[part_first_line + 5])
            {
                break;
            }
            part_first_line += 1;
        }
        traceln!(
            "parse3: Found part {part_first_line}..={}",
            part_first_line + 5
        );
        let mut part: Vec<&str> = lines[part_first_line..=part_first_line + 5]
            .iter()
            .map(|s| s.as_str().trim())
            .collect();
        part_first_line += 6;

        /// The current tick in THIS PART
        let mut tick = 0;
        // parse prelude and last char
        for l in part.iter_mut().take(6) {
            let (rem, string_name) = string_name()(l).unwrap();
            base_notes.push(string_name);
            let (rem, _) = super::char('|')(rem).unwrap();
            *l = rem;
            if !l.as_bytes().last().unwrap() == b'|' {
                panic!()
            };
            *l = &l[0..(l.len() - 1)];
        }

        let mut min_len = part.iter().map(|x| x.len()).min().unwrap_or(0);
        let mut multichar_carry = [false; 6];
        while tick < min_len {
            traceln!("parsing tick {tick}");
            let mut is_multichar = false;
            let mut s = 0;
            while s < 6 {
                traceln!(depth = 1, "remaining on string {s}: {}", part[s]);
                if s == 0 && part[s].as_bytes()[0] == b'|' {
                    traceln!(depth = 1, "encountered measure separator");
                    if multichar_carry[s] {
                        traceln!(depth = 1, "but there was a multichar carry");
                    } else {

                    }
                    let measure_start =
                        measures.last().map(|x| x.data_range.end() + 1).unwrap_or(0);
                    measures.push(Measure3 {
                        data_range: measure_start..=ticks.len().wrapping_sub(1) as u32,
                    });

                    part.iter_mut()
                        .for_each(|mut string| *string = &string[1..]); // TODO: maybe debugassert here that it is indeed a measure separator

                    traceln!(
                        depth = 1,
                        "remaining on string {s}: after fixup:{}",
                        part[s]
                    );
                }

                let len_before = part[s].len();
                let (res, te) = tab_element(part[s], drop).unwrap();
                let self_multichar = len_before - res.len() > 1;
                is_multichar |= self_multichar;
                part[s] = res;
                ticks.push(te);
                if multichar_carry[s] {
                    if !self_multichar && tick != 0 {
                        if let TabElement::Rest = ticks[ticks.len() - 7] {
                            let len = ticks.len();
                            ticks.swap(len - 7, len - 1);
                            traceln!(
                                depth = 2,
                                "swapped {:?} for {:?} on string {s}",
                                ticks[ticks.len() - 7],
                                ticks[ticks.len() - 1]
                            );
                            ticks.pop();
                            traceln!(
                                depth = 2,
                                "we fixed the last tick but now we have to parse this string again"
                            );
                            multichar_carry[s] = false;
                            continue;
                        }
                    }
                }
                s += 1;
            }
            if is_multichar {
                traceln!(
                    "tick {tick}/{min_len} was marked as multichar, so we take one less tick."
                );
                min_len -= 1;
            }
            multichar_carry = [is_multichar; 6];
            traceln!(
                depth = 1,
                "state after parsing tick {tick}/{min_len}:\n{}",
                dump_tracks_stream(&ticks)
            );
            tick += 1;
        }
        // finished parsing part
        traceln!("Finished part\n{}", dump_tracks_stream(&ticks));
    }
}

fn dump_tracks_stream(input: &[TabElement]) -> String {
    let input_len = input.len();
    let mut tracks: [Vec<RawTick>; 6] =
        std::array::from_fn(|_| Vec::with_capacity(input_len / 6 + 1));
    for (i, element) in input.iter().enumerate() {
        tracks[i % 6].push(RawTick {
            element: element.clone(),
        });
    }
    let h = HashMap::new();
    dump_tracks(&tracks, &h)
}

#[test]
pub fn test_parse3() {
    let example_score = r#"
e|--12-12|--12-12|
B|3------|3------|
G|-6-3-3-|-6-3-3-|
D|-------|-------|
A|-------|-------|
E|-----9-|-----9-|
"#;
    //  let parser2= Parser2{track_measures:false,track_sections:false};
    //  let time_parser2=Instant::now();
    //  parser2.parse(example_score.lines()).unwrap();
    //  println!("Parser2 took: {:?}", time_parser2.elapsed());

    let lines = &example_score
        .lines()
        .map(|x| x.to_string())
        .collect::<Vec<String>>();
    let time_parser3 = Instant::now();
    parse3(lines);
    println!("Parser3 took: {:?}", time_parser3.elapsed());
    insta::assert_debug_snapshot!(parse3(
        &example_score
            .lines()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
    ));
}
