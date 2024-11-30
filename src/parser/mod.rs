pub mod parser2;
mod parser3;
#[cfg(test)]
mod parser_tests;

use std::{cmp::max, collections::HashMap, ops::RangeInclusive};

use parser2::BendTargets;

use crate::{digit_cnt_u8, digit_cnt_usize, rlen};

#[derive(Debug, PartialEq)]
pub enum Section {
    Part { part: [Partline; 6] },
    Comment(String),
}

fn comment_line(s: &str) -> Result<(&str, &str), &str> {
    if s.len() < 2 || &s[0..2] != "//" {
        return Err(s);
    }
    let mut len = 0;
    for c in s.chars() {
        if c == '\n' || c == '\r' {
            break;
        }
        len += 1;
    }
    Ok((&s[len..], &s[0..len]))
}

#[derive(Debug, PartialEq, Clone)]
pub struct Partline {
    pub string_name: char,
    /// which measures originate from this partline in the measure buf of string_name
    pub measures: RangeInclusive<usize>,
}
impl Partline {
    /// Returns the measure count of this partline
    pub fn len(&self) -> usize {
        rlen(&self.measures)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
/// like `e|--------------4-----------|-----0--------------5-----|`
/// If called with append_to, the returned Partline will have no measures itself
/// TODO: make this know less by using callbacks
fn partline<'a>(
    s: &'a str,
    parent_line_idx: usize,
    start_source_offset: u32,
    string_buf: &mut Vec<RawTick>,
    string_measure_buf: &mut Vec<Measure>,
    string_offsets_buf: &mut Vec<u32>,
    bend_targets: &mut BendTargets,
    line_in_part: usize,
    track_measures: bool,
) -> Result<(&'a str, Partline), &'a str> {
    let (rem, string_name) = string_name()(s)?;
    let (mut rem, _) = char('|')(rem)?;
    let mut last_parsed_idx: u32 = 1;
    let mut measures = string_measure_buf.len()..=string_measure_buf.len();

    while !rem.is_empty() {
        let mut measure = Measure {
            content: string_buf.len()..=string_buf.len(),
            parent_line: parent_line_idx,
            index_on_parent_line: rlen(&measures),
        };
        loop {
            let rl_before = rem.len() as u32;
            let Ok(x) = tab_element(rem, |to| {
                bend_targets.insert((line_in_part as u8, string_buf.len() as u32), to);
            }) else {
                break;
            };
            rem = x.0;
            last_parsed_idx += rl_before - rem.len() as u32; // multichar frets
            string_buf.push(RawTick { element: x.1 });
            string_offsets_buf.push(start_source_offset + last_parsed_idx);
            if track_measures {
                measure.extend_1();
            }
        }
        if track_measures {
            measure.content = *measure.content.start()..=measure.content.end() - 1;
            string_measure_buf.push(measure);
            measures = *measures.start()..=measures.end() + 1;
        }
        rem = char('|')(rem)?.0;
        last_parsed_idx += 1;
    }
    // off by one: because we are using inclusive ranges, for example the first line, with only 1
    // measure, would be 0..=1 but we want it to be 0..=0
    if track_measures {
        measures = *measures.start()..=measures.end() - 1
    };
    Ok((
        rem,
        Partline {
            string_name,
            measures,
        },
    ))
}

/// A staff of a single string.
/// like `|--------------4-----------|`
/// The string it is on is encoded out-of-band
#[derive(Debug, PartialEq, Clone)]
pub struct Measure {
    /// The indices of the track this measure is on which belong to this measure
    pub content: RangeInclusive<usize>,
    pub parent_line: usize,
    pub index_on_parent_line: usize,
}

impl Measure {
    pub fn extend_1(&mut self) {
        self.content = *self.content.start()..=self.content.end() + 1
    }
    pub fn pop_1(&mut self) {
        self.content = *self.content.start()..=self.content.end() - 1
    }
    pub fn len(&self) -> usize {
        rlen(&self.content)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get_content<'a>(&self, string_buf: &'a [RawTick]) -> &'a [RawTick] {
        &string_buf[self.content.clone()]
    }
    pub fn print_pretty_string(
        &self,
        string_buf: &[RawTick],
        track_idx: u8,
        bend_targets: &BendTargets,
    ) -> String {
        dump_ticks(&string_buf[self.content.clone()], track_idx, bend_targets)
    }
}
fn char(c: char) -> impl Fn(&str) -> Result<(&str, char), &str> {
    move |s: &str| match s.chars().next() {
        Some(cc) if cc == c => Ok((&s[1..], c)),
        _ => Err(s),
    }
}
pub fn dump_ticks(buf: &[RawTick], track_idx: u8, bend_offsets: &BendTargets) -> String {
    let mut pretty = String::new();
    for (tick_idx, x) in buf.iter().enumerate() {
        match x.element {
            TabElement::Fret(x) => pretty += &x.to_string(),
            TabElement::Rest => pretty += "-",
            TabElement::DeadNote => pretty += "x",
            TabElement::FretBend(x) => pretty += &format!("{x}b"),
            TabElement::FretBendTo(x) => {
                let y = bend_offsets
                    .get(&(track_idx, tick_idx as u32))
                    .expect("Unreachable: FretBendTo without target");
                pretty += &format!("{x}b{y}");
            }
        }
    }
    pretty
}

/// horribly inefficient, for debugging only
pub fn dump_tracks(tracks: &[Vec<RawTick>; 6], bend_targets: &BendTargets) -> String {
    let tick_cnt = tracks.iter().map(|x| x.len()).max().unwrap();
    let mut bufs = vec![String::new(); 6];
    for track in 0..6 {
        let mut i = 0;
        while i < tick_cnt {
            if i >= tracks[track].len() {
                break;
            };
            let tick_len = tracks
                .iter()
                .enumerate()
                .filter_map(|(track_idx, x)| x.get(i).map(|x| ((track_idx as u8, i as u32), x)))
                .map(|(pos, x)| x.element.repr_len(bend_targets, &pos))
                .max()
                .unwrap() as usize;
            match tracks[track][i].element {
                TabElement::Fret(x) => bufs[track].push_str(&format!("{x:<0$}", tick_len)),
                TabElement::Rest => {
                    bufs[track].push_str(&format!("{1:<0$}", tick_len, "-"));
                }
                TabElement::DeadNote => {
                    bufs[track].push_str(&format!("{1:<0$}", tick_len, "x"));
                }
                TabElement::FretBend(x) => bufs[track].push_str(&format!("{x:<0$}b", tick_len - 1)),
                TabElement::FretBendTo(x) => {
                    let y = bend_targets
                        .get(&(track as u8, i as u32))
                        .expect("Unreachable: FretBendTo without target");
                    bufs[track].push_str(&format!("{1:<0$}", tick_len, format!("{x}b{y}")))
                }
            };
            i += 1
        }
        bufs[track].push('\n');
    }
    bufs.concat()
}

fn string_name() -> impl Fn(&str) -> Result<(&str, char), &str> {
    move |s: &str| match s.chars().next() {
        Some(c) if c.is_alphabetic() => Ok((&s[1..], c)),
        _ => Err(s),
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum TabElement {
    Fret(u8),
    FretBend(u8),
    FretBendTo(u8),
    Rest,
    DeadNote,
}

#[inline(always)]
fn numeric(s: &str) -> Result<(&str, u8), &str> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !matches!(bytes[i], 48..=58) {
            break;
        }
        i += 1;
    }
    if i == 0 {
        return Err(s);
    };
    let parsed: u8 = bytes[0..i]
        .iter()
        .rev()
        .map(|x| x - 48)
        .enumerate()
        .map(|(idx, x)| 10u8.pow(idx as u32) * x)
        .sum();
    Ok((&s[i..], parsed))
}
fn tab_element(s: &str, set_bend_target: impl FnOnce(u8)) -> Result<(&str, TabElement), &str> {
    let bytes = s.as_bytes();
    match bytes.first() {
        Some(b'-') => Ok((&s[1..], TabElement::Rest)),
        Some(b'x') => Ok((&s[1..], TabElement::DeadNote)),

        Some(48..=58) => {
            // FIXME: this will check for errors even if we know there is one char
            // probably should optimize
            let (res, num) = numeric(s).unwrap();
            let bytes = res.as_bytes();
            if let Some(b'b') = bytes.first() {
                //println!("we have a bend");
                if let Some(48..=58) = bytes.get(1) {
                    let (res, bend_target) = numeric(&res[1..]).unwrap();
                    //println!("bendTo, will return {num}b{bend_target}, res={res}");
                    set_bend_target(bend_target);
                    return Ok((res, TabElement::FretBendTo(num)));
                }
                //println!("bend: will return `{}`, FretBend({num})", &res[1..]);
                return Ok((&res[1..], TabElement::FretBend(num)));
            }
            Ok((res, TabElement::Fret(num)))
        }
        Some(_) | None => Err(s),
    }
}

impl TabElement {
    #[inline(always)]
    pub fn repr_len(&self, bend_targets: &BendTargets, pos: &(u8, u32)) -> u8 {
        match self {
            TabElement::Fret(x) => digit_cnt_u8(x),
            TabElement::FretBend(x) => digit_cnt_u8(x) + 1,
            TabElement::FretBendTo(x) => {
                let y = bend_targets
                    .get(pos)
                    .expect("TabElement::repr_len: FretBendTo without target");
                digit_cnt_u8(x) + 1 + digit_cnt_u8(y)
            }
            TabElement::Rest => 1,
            TabElement::DeadNote => 1,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct RawTick {
    pub element: TabElement,
}
