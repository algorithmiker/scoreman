pub mod parser2;
mod parser3;
#[cfg(test)]
mod parser_tests;

use std::cmp::max;

use nom::{
    branch::alt,
    bytes::complete::is_not,
    character::complete::{char, digit1, none_of},
    error::{context, VerboseError},
    sequence::preceded,
    IResult, Parser,
};

use nom_supreme::tag::complete::tag;

#[derive(Debug, PartialEq)]
pub struct Score(pub Vec<Section>);

type VerboseResult<Input, Parsed> = IResult<Input, Parsed, VerboseError<Input>>;

#[derive(Debug, PartialEq)]
pub enum Section {
    Part {
        part: [Partline; 6],
    },
    Comment(String),
}

fn comment_line(s: &str) -> VerboseResult<&str, &str> {
    preceded(tag("//"), is_not("\n\r")).parse(s)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Partline {
    pub string_name: char,
    pub measures: Vec<Measure>,
}

/// like `e|--------------4-----------|-----0--------------5-----|`
fn partline(s: &str, parent_line_idx: usize) -> VerboseResult<&str, Partline> {
    let (rem, string_name) = none_of("|").parse(s)?;
    let (mut rem, _) = char('|').parse(rem)?;
    let mut parsed_len = 2;
    let mut measures = Vec::with_capacity(2);
    while !rem.is_empty() {
        let mut measure = Measure {
            content: Vec::with_capacity(4),
            parent_line: parent_line_idx,
            index_on_parent_line: max(measures.len(), 1) - 1,
        };
        loop {
            let Ok(x) = tab_element(rem) else { break };
            rem = x.0;
            measure.content.push(RawTick {
                element: x.1,
                parent_line: parent_line_idx,
                idx_on_parent_line: parsed_len,
            });
            parsed_len += 1;
        }
        measures.push(measure);
        rem = char('|').parse(rem)?.0;
        parsed_len += 1;
    }
    Ok((
        rem,
        Partline {
            string_name,
            measures,
        },
    ))
    //context(
    //    "Partline",
    //    tuple((none_of("|"), terminated(many1(measure), char('|')))).map(
    //        |(string_name, staffs)| Partline {
    //            string_name,
    //            staffs,
    //        },
    //    ),
    //)
    //.parse(s)
}

/// A staff of a single string.
/// like `|--------------4-----------|`
#[derive(Debug, PartialEq, Clone)]
pub struct Measure {
    pub content: Vec<RawTick>,
    pub parent_line: usize,
    pub index_on_parent_line: usize,
}

impl Measure {
    pub fn print_pretty_string(&self) -> String {
        let mut pretty = String::new();
        for x in &self.content {
            match x.element {
                TabElement::Fret(x) => pretty += &x.to_string(),
                TabElement::Rest => pretty += "-",
                TabElement::DeadNote => pretty += "x",
            }
        }
        pretty
    }
}
#[inline]
fn tab_element(s: &str) -> VerboseResult<&str, TabElement> {
    use TabElement::*;
    context(
        "TabElement",
        alt((
            char('-').map(|_| Rest),
            char('x').map(|_| DeadNote),
            digit1.map(|x: &str| {
                Fret(x.parse::<u8>().unwrap_or_else(|_| {
                    panic!("failed to parse {x} to a fret position, in Measure")
                }))
            }),
        )),
    )
    .parse(s)
}

#[derive(Debug, PartialEq, Clone)]
pub struct RawTick {
    pub element: TabElement,
    pub parent_line: usize,
    pub idx_on_parent_line: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TabElement {
    Fret(u8),
    Rest,
    DeadNote,
}
