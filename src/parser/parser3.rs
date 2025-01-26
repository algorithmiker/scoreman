use std::cmp::max;
use std::io::Read;
use std::ops::RangeInclusive;

use super::{numeric, string_name};
use crate::{
    backend::{errors::backend_error::BackendError, muxml2::Muxml2TabElement},
    debugln, time, traceln,
};

pub fn line_is_valid(line: &str) -> bool {
    let line = line.trim();
    let first_is_alphanumeric = line.chars().next().map(|x| x.is_alphanumeric()).unwrap_or(false);
    let last_is_measure_end = line.as_bytes().last().map(|x| *x == b'|').unwrap_or(false);
    let ret = first_is_alphanumeric && last_is_measure_end;
    traceln!("line_is_valid({line}) -> {ret}");
    ret
}
#[derive(Debug)]
pub struct Measure3 {
    pub data_range: RangeInclusive<u32>,
}

#[derive(Debug, Default)]
pub struct Parse3Result<'a> {
    /// This is not a [std::Result] because the tick stream and offsets are needed for printing the error
    pub error: Option<BackendError<'a>>,
    pub tick_stream: Vec<TabElement3>,
    pub measures: Vec<Measure3>,
    pub base_notes: Vec<char>,
    /// The line on which the n-th section begins and the index of the first tick in that section.
    /// This provides enough information to restore from where we have read an individual tick.
    pub offsets: Vec<(u32, u32)>,
}

impl Parse3Result<'_> {
    pub fn new() -> Self {
        Self::default()
    }
    fn dump_tracks(&self) -> String {
        let stream_len = self.tick_stream.len();
        let tick_cnt = stream_len / 6;
        if stream_len % 6 != 0 {
            panic!("Invalid stream")
        }
        let mut bufs = vec![String::new(); 6];
        for tick in 0..tick_cnt {
            let max_width =
                (0..6).map(|x| dumb_repr_len(&self.tick_stream[tick * 6 + x])).max().unwrap()
                    as usize;
            for s in 0..6 {
                use TabElement3::*;
                match self.tick_stream[tick * 6 + s] {
                    Fret(x) => bufs[s].push_str(&format!("{x:<0$}", max_width)),
                    Rest => bufs[s].push_str(&format!("{1:<0$}", max_width, "-")),
                    DeadNote => bufs[s].push_str(&format!("{1:<0$}", max_width, "x")),
                    Slide => bufs[s].push_str(&format!("{1:<0$}", max_width, "/")),
                    Bend => bufs[s].push_str(&format!("{1:<0$}", max_width, "b")),
                    HammerOn => bufs[s].push_str(&format!("{1:<0$}", max_width, "h")),
                    Pull => bufs[s].push_str(&format!("{1:<0$}", max_width, "p")),
                    Release => bufs[s].push_str(&format!("{1:<0$}", max_width, "r")),
                }
            }
        }
        bufs.iter_mut().for_each(|x| x.push('\n'));
        bufs.concat()
    }
}

fn dumb_repr_len(x: &TabElement3) -> u32 {
    use TabElement3::*;
    match x {
        Fret(x) => max(x, &1).ilog10() + 1,
        Bend | HammerOn | DeadNote | Pull | Slide | Rest | Release => 1,
    }
}
pub fn parse3(lines: &[String]) -> Parse3Result {
    let mut r = Parse3Result::new();
    let mut part_first_line = 0;
    while part_first_line + 5 < lines.len() {
        // find a part
        while part_first_line + 5 < lines.len() {
            if line_is_valid(&lines[part_first_line]) && line_is_valid(&lines[part_first_line + 5])
            {
                break;
            }
            part_first_line += 1;
        }
        // PRERELEASE: this loop ^ fails if there is extra content after the last part. find a good way to fix that.
        traceln!("parse3: Found part {part_first_line}..={}", part_first_line + 5);
        r.offsets.push((part_first_line as u32, r.tick_stream.len() as u32));
        let mut part: Vec<&str> = lines[part_first_line..=part_first_line + 5]
            .iter()
            .map(|s| s.as_str().trim())
            .collect(); // TODO: check if this is slow

        // The current tick in THIS PART
        let mut tick = 0;
        // parse prelude and last char
        for (line_idx, line) in part.iter_mut().enumerate() {
            let Ok((rem, string_name)) = string_name()(line) else {
                // TODO: error for invalid string name
                let char = line.chars().next().unwrap_or('\0');
                r.error = Some(BackendError::parse3_invalid_character(line_idx as u32, 0, char));
                return r;
            };
            r.base_notes.push(string_name);
            let (rem, _) = super::char('|')(rem).unwrap();
            *line = rem;
            if !line.as_bytes().last().unwrap() == b'|' {
                r.error = Some(BackendError::no_closing_barline(part_first_line + line_idx));
                return r;
            };
            *line = &line[0..(line.len() - 1)];
        }

        let mut tick_cnt_est = part[0].len();
        while tick < tick_cnt_est {
            traceln!("parsing tick {tick}");
            let mut is_multichar = false;
            let mut is_multi_on = [false; 6];
            let mut s = 0;
            while s < 6 {
                traceln!(depth = 1, "remaining on string {s}: {}", part[s]);
                if s == 0 && part[s].as_bytes()[0] == b'|' {
                    traceln!(depth = 1, "encountered measure separator");
                    let measure_start =
                        r.measures.last().map(|x| x.data_range.end() + 1).unwrap_or(0);
                    r.measures.push(Measure3 {
                        data_range: measure_start..=r.tick_stream.len().wrapping_sub(1) as u32,
                    });
                    part.iter_mut().for_each(|mut string| *string = &string[1..]); // TODO: maybe debugassert here that it is indeed a measure separator
                    tick_cnt_est -= 1;
                    traceln!(depth = 1, "remaining on string {s}: after fixup:{}", part[s]);
                }

                let len_before = part[s].len();
                let Ok((res, te)) = tab_element3(part[s]) else {
                    let (line, char) =
                        source_location_while_parsing(&r, lines, part_first_line as u32, s as u32);
                    let invalid_src = part[s].chars().next().unwrap_or('\0');
                    r.error = Some(BackendError::parse3_invalid_character(line, char, invalid_src));
                    return r;
                };
                let tab_element_len = len_before - res.len();
                is_multichar |= tab_element_len > 1;
                is_multi_on[s] = tab_element_len > 1;
                part[s] = res;
                r.tick_stream.push(te);
                s += 1;
            }
            if is_multichar {
                traceln!("tick {tick}/{tick_cnt_est} was marked as multichar, so we run fixup.");
                tick_cnt_est -= 1;
                for s in (0..6) {
                    if is_multi_on[s] {
                        traceln!(depth = 1, "multi on {s}, skipping");
                        continue;
                    };
                    let elem = &r.tick_stream[r.tick_stream.len() - (6 - s)];
                    traceln!(depth = 1, "on string {s} we have {:?}", elem);
                    if let TabElement3::Rest = elem {
                        traceln!(depth = 2, "this is a rest so we try to parse the next element");
                        let len_before = part[s].len();
                        let next = tab_element3(part[s]).unwrap();
                        if len_before - next.0.len() > 1 {
                            panic!("multichar with next slot multichar too") // TODO error here
                        }
                        let len = r.tick_stream.len(); // to make the borrow checker happy about borrowing &mut and &
                        r.tick_stream[len - (6 - s)] = next.1;
                        part[s] = next.0;
                        traceln!(
                            depth = 1,
                            "replaced this Rest with {:?}",
                            r.tick_stream[r.tick_stream.len() - (6 - s)]
                        );
                    } else {
                        traceln!(depth = 2, "this is not a Rest, so we check the next element");
                        if part[s].chars().next().unwrap() == '-' {
                            traceln!(depth = 2, "next element is Rest so we skip it");
                            part[s] = &part[s][1..];
                        } else {
                            panic!("Not multichar but both slots are filled"); // eg. 3x under a 12 is invalid
                        }
                    }
                }
            }
            traceln!(depth = 1, "data state after parsing tick {tick}\n{}", r.dump_tracks());
            traceln!(depth = 1, "source state after parsing tick:\n{}", dump_source(&part));
            tick += 1;
        }

        let measure_start = r.measures.last().map(|x| x.data_range.end() + 1).unwrap_or(0);
        r.measures.push(Measure3 {
            data_range: measure_start..=r.tick_stream.len().wrapping_sub(1) as u32,
        });
        // finished parsing part
        traceln!("Finished part\n{}", r.dump_tracks());

        part_first_line += 6;
    }
    r
}

pub fn source_location_while_parsing(
    r: &Parse3Result, lines: &[String], part_first_line: u32, line_in_part: u32,
) -> (u32, u32) {
    let actual_line = part_first_line + line_in_part;
    traceln!("expecting the error to be on line {actual_line}");
    let mut offset_on_line = 2; // e|
    let mut idx_in_stream = (r.offsets.last().map(|x| x.1).unwrap_or(0) + line_in_part) as usize;
    while idx_in_stream < r.tick_stream.len() {
        let element = &r.tick_stream[idx_in_stream];
        traceln!(depth = 1, "adding offset ({}) of element {:?}", dumb_repr_len(element), element);
        offset_on_line += dumb_repr_len(element);
        idx_in_stream += 6;
    }
    offset_on_line += 1; // because the location refers to the offset of the tick that was not parsed
    traceln!("expecting the error to be at character {offset_on_line}");
    (actual_line, offset_on_line)
}
pub fn source_location_from_stream(
    r: &Parse3Result, lines: &[&str], tick_location: u32,
) -> (u32, u32) {
    let section = r
        .offsets
        .binary_search_by_key(&tick_location, |x| x.1)
        .unwrap_or_else(|x| x.saturating_sub(1));
    traceln!("source_location_from_stream: expected to be in section {section}");
    let idx_in_part = tick_location - r.offsets[section].1;
    traceln!("this is the {idx_in_part}th element in the part");
    let line_in_part = idx_in_part % 6;
    let actual_line = r.offsets[section].0 + line_in_part;
    traceln!("expecting the error to be on line {actual_line}");
    let mut offset_on_line = 2; // e|
    let mut idx_in_stream = (r.offsets[section].1 + line_in_part) as usize;
    let tick_location = tick_location as usize;
    while idx_in_stream < tick_location {
        traceln!(depth = 1, "adding offset of tick {:?}", r.tick_stream[idx_in_stream]);
        offset_on_line += dumb_repr_len(&r.tick_stream[idx_in_stream]);
        idx_in_stream += 6;
    }
    offset_on_line += 1; // because the location refers to the offset of the tick that was not parsed
    traceln!("expecting the error to be at character {offset_on_line}");
    (actual_line, offset_on_line)
}
pub fn dump_source(input: &Vec<&str>) -> String {
    use itertools::Itertools;
    input.iter().join("\n")
}

#[derive(Debug, PartialEq, Clone)]
pub enum TabElement3 {
    Fret(u8),
    Rest,
    DeadNote,
    Bend,
    HammerOn,
    Pull,
    Release,
    Slide,
}

// PRERELEASE: parse vibrato
fn tab_element3(s: &str) -> Result<(&str, TabElement3), &str> {
    let bytes = s.as_bytes();
    match bytes.first() {
        Some(b'-') => Ok((&s[1..], TabElement3::Rest)),
        Some(b'x') => Ok((&s[1..], TabElement3::DeadNote)),
        Some(48..=58) => {
            let (res, num) = numeric(s)?;
            Ok((res, TabElement3::Fret(num)))
        }
        Some(b'b') => Ok((&s[1..], TabElement3::Bend)),
        Some(b'h') => Ok((&s[1..], TabElement3::HammerOn)),
        Some(b'p') => Ok((&s[1..], TabElement3::Pull)),
        Some(b'r') => Ok((&s[1..], TabElement3::Release)),
        Some(b'/') | Some(b'\\') => Ok((&s[1..], TabElement3::Slide)),
        Some(_) | None => Err(s),
    }
}
#[test]
pub fn test_parse3() {
    use std::time::Instant;
    let example_score = r#"
e|--12-12|--12-12|--12-12|
B|3------|3------|3----11|
G|-6-3-3-|-6-3-3-|-6-3-11|
D|-------|-------|-----11|
A|-------|-------|-----11|
E|-----9-|-----9-|-----11|

// This is a comment!

e|--12-12|--12-12|
B|3------|3------|
G|-6-3-3-|-6-3-3-|
D|-------|-------|
A|-------|-------|
E|-----9-|-----9-|
"#;
    let lines = &example_score.lines().map(|x| x.to_string()).collect::<Vec<String>>();
    let time_parser3 = Instant::now();
    let parse3_result = parse3(lines);
    println!("Parser3 took: {:?}", time_parser3.elapsed());
    insta::assert_snapshot!(parse3_result.dump_tracks());
    insta::assert_debug_snapshot!(parse3_result);
}
