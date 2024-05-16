pub mod parser2;

#[cfg(test)]
mod parser_tests;
use nom::{
    branch::alt,
    bytes::complete::is_not,
    character::complete::{char, digit1, none_of},
    error::{context, VerboseError},
    multi::many1,
    sequence::{preceded, terminated, tuple},
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
        begin_line_idx: usize,
        end_line_idx: usize,
    },
    Comment(String),
}

fn comment_line(s: &str) -> VerboseResult<&str, &str> {
    preceded(tag("//"), is_not("\n\r")).parse(s)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Partline {
    pub string_name: char,
    pub staffs: Vec<Measure>,
}

/// like `e|--------------4-----------|-----0--------------5-----|`
fn partline(s: &str) -> VerboseResult<&str, Partline> {
    context(
        "Partline",
        tuple((none_of("|"), terminated(many1(measure), char('|')))).map(
            |(string_name, staffs)| Partline {
                string_name,
                staffs,
            },
        ),
    )
    .parse(s)
}

/// A staff of a single string.
/// like `|--------------4-----------|`
#[derive(Debug, PartialEq, Clone)]
pub struct Measure {
    pub content: Vec<TabElement>,
    pub parent_line: Option<usize>,
    pub index_on_parent_line: Option<usize>,
}

impl Measure {
    fn from_content(content: Vec<TabElement>) -> Measure {
        Measure {
            content,
            parent_line: None,
            index_on_parent_line: None,
        }
    }

    pub fn print_pretty_string(&self) -> String {
        let mut pretty = String::new();
        for x in &self.content {
            match x {
                TabElement::Fret(x) => pretty += &x.to_string(),
                TabElement::Rest => pretty += "-",
                TabElement::DeadNote => pretty += "x",
            }
        }
        pretty
    }
}

fn measure(s: &str) -> VerboseResult<&str, Measure> {
    use TabElement::*;
    context(
        "Measure",
        preceded(
            char('|'),
            many1(context(
                "TabElement",
                alt((
                    char('-').map(|_| Rest),
                    char('x').map(|_| DeadNote),
                    digit1.map(|x: &str| {
                        Fret(x.parse::<u16>().unwrap_or_else(|_| {
                            panic!("failed to parse {x} to a fret position, in Measure")
                        }))
                    }),
                )),
            )),
        ),
    )
    .map(Measure::from_content)
    .parse(s)
}

#[derive(Debug, PartialEq, Clone)]
pub enum TabElement {
    Fret(u16),
    Rest,
    /// on which string:
    DeadNote,
}
