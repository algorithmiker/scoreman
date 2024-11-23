pub mod parser2;
mod parser3;
#[cfg(test)]
mod parser_tests;

use std::ops::RangeInclusive;

use crate::rlen;

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
fn partline<'a>(
    s: &'a str,
    parent_line_idx: usize,
    start_source_offset: u32,
    string_buf: &mut Vec<RawTick>,
    string_measure_buf: &mut Vec<Measure>,
    string_offsets_buf: &mut Vec<u32>,
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
            let Ok(x) = tab_element(rem) else { break };
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
    pub fn print_pretty_string(&self, string_buf: &[RawTick]) -> String {
        let mut pretty = String::new();
        for x in self.content.clone() {
            match string_buf[x].element {
                TabElement::Fret(x) => pretty += &x.to_string(),
                TabElement::Rest => pretty += "-",
                TabElement::DeadNote => pretty += "x",
            }
        }
        pretty
    }
}
fn char(c: char) -> impl Fn(&str) -> Result<(&str, char), &str> {
    move |s: &str| match s.chars().next() {
        Some(cc) if cc == c => Ok((&s[1..], c)),
        _ => Err(s),
    }
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
    Rest,
    DeadNote,
}
fn tab_element(s: &str) -> Result<(&str, TabElement), &str> {
    let bytes = s.as_bytes();
    match bytes.first() {
        Some(b'-') => Ok((&s[1..], TabElement::Rest)),
        Some(b'x') => Ok((&s[1..], TabElement::DeadNote)),
        Some(48..=58) => {
            let mut len = 1;
            // 123a
            for cc in &bytes[1..] {
                if !matches!(cc, 48..=58) {
                    break;
                }
                len += 1;
            }
            let parsed: u8 = bytes[0..len]
                .iter()
                .rev()
                .map(|x| x - 48)
                .enumerate()
                .map(|(idx, x)| 10u8.pow(idx as u32) * x)
                .sum();
            debug_assert_eq!(Ok(parsed), s[0..len].parse());
            Ok((&s[len..], TabElement::Fret(parsed)))
        }
        Some(_) | None => Err(s),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct RawTick {
    pub element: TabElement,
}
