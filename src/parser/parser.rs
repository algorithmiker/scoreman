use tracing::{debug, span, trace, Level};

use super::{
    string_name,
    tab_element::{self, tab_element3, TabElement},
};
use crate::{backend::errors::backend_error::BackendError, parser::tab_element::TabElementError};
use std::ops::RangeInclusive;

pub fn line_is_valid(line: &str) -> bool {
    let line = line.trim();
    let mut chars = line.chars();
    let first_is_alphanumeric = chars.next().map(|x| x.is_alphanumeric()).unwrap_or(false);
    let second_is_measure_sep = chars.next().map(|x| x == '|').unwrap_or(false);
    let last_is_measure_end = line.ends_with('|');
    let ret = first_is_alphanumeric && second_is_measure_sep && last_is_measure_end;
    trace!(line, verdict = ret, "line_is_valid");
    ret
}

#[derive(Debug)]
pub struct Measure {
    pub data_range: RangeInclusive<u32>,
}
impl Measure {
    pub fn from(range: RangeInclusive<u32>) -> Self {
        Self { data_range: range }
    }
}

#[derive(Debug, Default)]
pub struct ParseResult {
    /// This is not a [Result] because we want to preserve the partial parse state, eg. for fixup or recovery
    pub error: Option<BackendError>,
    pub tick_stream: Vec<TabElement>,
    pub measures: Vec<Measure>,
    pub base_notes: Vec<char>,
    /// The line on which the n-th section begins and the index of the first tick in that section.
    /// This provides enough information to restore from where we have read an individual tick.
    pub offsets: Vec<(u32, u32)>,
}

impl ParseResult {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn dump_tracks(&self) -> String {
        let stream_len = self.tick_stream.len();
        let tick_cnt = stream_len / 6;
        debug_assert_eq!(stream_len % 6, 0);
        let mut bufs = vec![String::new(); 6];
        for tick in 0..tick_cnt {
            let max_width =
                (0..6).map(|x| self.tick_stream[tick * 6 + x].repr_len()).max().unwrap() as usize;
            for (s, buf) in bufs.iter_mut().enumerate() {
                use tab_element::TabElement::*;
                let to_padded = |c: char| format!("{1:<0$}", max_width, c);
                match self.tick_stream[tick * 6 + s] {
                    Fret(x) => buf.push_str(&format!("{x:<0$}", max_width)),
                    Rest => buf.push_str(&to_padded('-')),
                    DeadNote => buf.push_str(&to_padded('x')),
                    Slide => buf.push_str(&to_padded('/')),
                    Bend => buf.push_str(&to_padded('b')),
                    HammerOn => buf.push_str(&to_padded('h')),
                    Pull => buf.push_str(&to_padded('p')),
                    Release => buf.push_str(&to_padded('r')),
                    Vibrato => buf.push_str(&to_padded('~')),
                }
            }
        }
        bufs.iter_mut().for_each(|x| x.push('\n'));
        bufs.concat()
    }
}

pub fn parse<T: AsRef<str>>(lines: &[T]) -> ParseResult {
    let mut r = ParseResult::new();
    let mut part_first_line = 0;
    'outer: loop {
        // find a part
        loop {
            if part_first_line + 5 >= lines.len() {
                break 'outer;
            }
            if line_is_valid(lines[part_first_line].as_ref())
                && line_is_valid(lines[part_first_line + 5].as_ref())
            {
                break;
            }
            part_first_line += 1
        }
        let range = part_first_line..=part_first_line + 5;
        let _part = span!(Level::DEBUG, "parsing part", ?range);
        let _part = _part.enter();
        r.offsets.push((part_first_line as u32, r.tick_stream.len() as u32));
        let mut part: Vec<&str> = lines[range].iter().map(|s| s.as_ref().trim()).collect(); // TODO: check if this is slow

        // The current tick in THIS PART
        let mut tick = 0;
        // parse prelude and last char
        for (line_idx, line) in part.iter_mut().enumerate() {
            let Ok((rem, string_name)) = string_name()(line) else {
                r.error = Some(BackendError::invalid_string_name(part_first_line + line_idx));
                return r;
            };
            r.base_notes.push(string_name);
            let Ok((rem, _)) = super::char('|')(rem) else {
                r.error = Some(BackendError::invalid_string_name(part_first_line + line_idx));
                return r;
            };
            *line = rem;
            if !line.ends_with('|') {
                r.error = Some(BackendError::no_closing_barline(part_first_line + line_idx));
                return r;
            };
            *line = &line[0..(line.len() - 1)];
        }

        let mut tick_cnt_est = part[0].len();
        while tick < tick_cnt_est {
            let s = span!(Level::TRACE, "parsing tick", tick);
            let _s = s.enter();
            let (mut is_multichar, mut is_multi_on) = (false, [false; 6]);
            for s in 0..6 {
                trace!(part = part[s], "remaining on string {s}:");
                if s == 0 && part[s].starts_with("|") {
                    trace!("encountered measure separator");
                    let measure_start =
                        r.measures.last().map(|x| x.data_range.end() + 1).unwrap_or(0);
                    r.measures.push(Measure::from(
                        measure_start..=r.tick_stream.len().wrapping_sub(1) as u32,
                    ));
                    part.iter_mut().for_each(|string| *string = &string[1..]); // TODO: maybe debugassert here that it is indeed a measure separator
                    tick_cnt_est -= 1;
                    trace!(part = part[s], "remaining on string {s}: after fixup");
                }

                let len_before = part[s].len();
                let (res, te) = match tab_element3(part[s]) {
                    Ok(x) => x,
                    Err((_, err)) => {
                        let (line, char) =
                            source_location_while_parsing(&r, part_first_line as u32, s as u32);
                        if let Some(TabElementError::FretTooLarge) = err {
                            r.error = Some(BackendError::large_fret(line, char));
                        } else {
                            let invalid_src = part[s].chars().next();
                            let err = BackendError::invalid_character(line, char, invalid_src);
                            r.error = Some(err);
                        }
                        return r;
                    }
                };

                let tab_element_len = len_before - res.len();
                is_multichar |= tab_element_len > 1;
                is_multi_on[s] = tab_element_len > 1;
                part[s] = res;
                r.tick_stream.push(te);
            }
            if is_multichar {
                let _ms = span!(Level::DEBUG, "marked as multichar, running fixup", tick);
                let _ms = _ms.enter();
                tick_cnt_est -= 1;
                for s in 0..6 {
                    if is_multi_on[s] {
                        trace!("multi on {s}, skipping");
                        continue;
                    };
                    let elem_idx = r.tick_stream.len() - (6 - s);
                    let elem = &r.tick_stream[elem_idx];
                    let _s = span!(Level::DEBUG, "fixing up string", string = s, ?elem);
                    let _s = _s.enter();
                    if let TabElement::Rest = elem {
                        let _s2 = span!(
                            Level::TRACE,
                            "this is a rest so we try to parse the next element"
                        );
                        let _s2 = _s2.enter();
                        let len_before = part[s].len();
                        let next = tab_element3(part[s]).unwrap(); // TODO: the unwrap here is ICE,
                                                                   // should error instead
                        if len_before - next.0.len() > 1 {
                            let (m_line, m_char) = source_location_from_stream(&r, elem_idx as u32);
                            // just for a nicer error, show another multi line too
                            let other = ((0..6).find(|x| is_multi_on[*x]).unwrap()
                                + part_first_line) as u32;
                            r.error =
                                Some(BackendError::both_slots_multichar(m_line, m_char, other));
                            return r;
                        }
                        trace!(replacement = ?next.1, "replaced a rest");
                        let len = r.tick_stream.len(); // to make the borrow checker happy about borrowing &mut and &
                        r.tick_stream[len - (6 - s)] = next.1;
                        part[s] = next.0;
                    } else {
                        let _q =
                            span!(Level::TRACE, "this is not a Rest, so we check the next element");
                        let _q = _q.enter();
                        if part[s].starts_with("-") {
                            trace!("next element is Rest so we skip it");
                            part[s] = &part[s][1..];
                        } else {
                            let (line, char) = source_location_from_stream(&r, elem_idx as u32);
                            r.error = Some(BackendError::multi_both_slots_filled(line, char));
                            return r;
                        }
                    }
                }
            }
            trace!(tick, data = r.dump_tracks(), "data after parsing tick");
            trace!(source = dump_source(&part), "source state after parsing tick");
            tick += 1;
        }

        let measure_start = r.measures.last().map(|x| x.data_range.end() + 1).unwrap_or(0);
        r.measures.push(Measure {
            data_range: measure_start..=r.tick_stream.len().wrapping_sub(1) as u32,
        });
        // finished parsing part
        trace!(part = r.dump_tracks(), "Finished part");

        part_first_line += 6;
    }
    r
}

/// A specialized, faster [source_location_from_stream]
pub fn source_location_while_parsing(
    r: &ParseResult, part_first_line: u32, line_in_part: u32,
) -> (u32, u32) {
    let actual_line = part_first_line + line_in_part;
    trace!("expecting the error to be on line {actual_line}");
    let error_tick = r.tick_stream.len() / 6;
    // we aren't accounting for measures here, so sum of all the measure lines too
    let part_start = r.offsets.last().map(|x| x.1).unwrap_or(0);
    let mut measure_lines = 0;
    for measure in r.measures.iter().rev() {
        if measure.data_range.start() < &part_start {
            break;
        }
        measure_lines += 1;
    }
    trace!("need to account for {measure_lines} measure lines");
    let mut offset_on_line = 1 + measure_lines; // e|
    let start = (part_start / 6) as usize;
    for tick in start..error_tick {
        // take the maximum extent of this tick. we cannot just add up the local tick lengths because multichars on *other strings* would throw off the parser
        // -1-2-3-
        // -11b12- <- this would think that if there is an error on the first string, the extents before are just rest-1-2-3-rest, and report an incorrect location
        let remainder = &r.tick_stream[tick * 6..];
        //trace!(?remainder);
        let tick_width = remainder.iter().take(6).map(|x| x.repr_len()).max();
        let tick_width = tick_width.unwrap_or(0);
        trace!(offset = tick_width, tick, "adding offset for tick");
        offset_on_line += tick_width;
    }
    offset_on_line += 1; // because the location refers to the offset of the tick that was not parsed
    trace!("expecting the error to be at character idx {offset_on_line}");
    (actual_line, offset_on_line)
}

pub fn source_location_from_stream(r: &ParseResult, tick_location: u32) -> (u32, u32) {
    let section = r
        .offsets
        .binary_search_by_key(&tick_location, |x| x.1)
        .unwrap_or_else(|x| x.saturating_sub(1));
    trace!("source_location_from_stream: expected to be in section {section}");
    let part_start = r.offsets[section].1;
    let idx_in_part = tick_location - part_start;
    trace!("this is the {idx_in_part}th element in the part");
    let line_in_part = idx_in_part % 6;
    let actual_line = r.offsets[section].0 + line_in_part;
    trace!("expecting the error to be on line {actual_line}");

    let error_tick = (tick_location / 6) as usize;
    // we aren't accounting for measures here, so sum of all the measure lines to
    // search for all the measures in this part, and before the needle
    let last_measure = r
        .measures
        .binary_search_by_key(&tick_location, |x| x.data_range.end() + 1)
        .unwrap_or_else(|x| x);
    debug!("last measure we need to check: {last_measure} for needle {tick_location}");
    let mut measure_lines = 0;
    trace!("{:?}", r.measures);
    trace!("part start: {part_start}");
    for (m_idx, measure) in r.measures[0..last_measure].iter().enumerate().rev() {
        if measure.data_range.start() < &part_start {
            trace!("breaking at measure {m_idx}");
            break;
        }
        measure_lines += 1;
    }
    measure_lines += 1; // for last measure; which we cannot index with 0..=last_measure if we have only 1.
    trace!("need to account for {measure_lines} measure lines");
    let mut offset_on_line = 1 + measure_lines; // e|
    let start = (part_start / 6) as usize;
    for tick in start..error_tick {
        // take the maximum extent of this tick. we cannot just add up the local tick lengths because multichars on *other strings* would throw off the parser
        // -1-2-3-
        // -11b12- <- this would think that if there is an error on the first string, the extents before are just rest-1-2-3-rest, and report an incorrect location
        let remainder = &r.tick_stream[tick * 6..];
        //traceln!(depth = 1, "remainder: {remainder:?}");
        let tick_width = remainder.iter().take(6).map(|x| x.repr_len()).max();
        let tick_width = tick_width.unwrap_or(0);
        trace!(offset = tick_width, tick, "adding offset for tick");
        offset_on_line += tick_width;
    }
    trace!("expecting the error to be at character idx {offset_on_line}");
    (actual_line, offset_on_line)
}

pub fn dump_source(input: &Vec<&str>) -> String {
    use itertools::Itertools;
    std::iter::once(&"").chain(input.iter()).join("\n")
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
    let parse3_result = parse(lines);
    println!("Parser3 took: {:?}", time_parser3.elapsed());
    insta::assert_snapshot!(parse3_result.dump_tracks());
    insta::assert_debug_snapshot!(parse3_result);
}
