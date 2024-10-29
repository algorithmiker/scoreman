#![allow(dead_code, unused_variables)]

use crate::backend::{errors::backend_error::BackendError, muxml2::Muxml2TabElement};

use super::comment_line;
enum Parse3Element {
    CommentLine(String),
    Part(Vec<Parse3Tick>),
}

pub struct Parse3Tick {
    note: Muxml2TabElement,
    line: usize,
    measure: usize,
    line_offset: usize,
}
// this is just an experiment
pub fn parse3<'a>(lines: &[String]) -> Result<(), BackendError<'a>> {
    let mut parsed: Vec<Parse3Element> = vec![];
    for i in 0..lines.len() {
        let line = &lines[i];
        if line.is_empty() {
            continue;
        }
        if let Ok((rem, x)) = comment_line(line) {
            assert_eq!(rem, "");
            parsed.push(Parse3Element::CommentLine(x.to_owned()));
            continue;
        }
        // TODO nice error if incomplete part
        let next_6_l = &lines[i..i + 6];
        let max_chars = next_6_l.iter().map(|x| x.len()).max().unwrap();
        let strings: Vec<char> = (0..6)
            .map(|i| char::from(next_6_l[i].as_bytes()[0]))
            .collect();

        for i in 2..max_chars {
            for row in 0..6 {}
        }
    }
    Ok(())
}

pub fn tokenize3(inp: &str) {}

#[test]
#[ignore = "Parser3 is experimental"]
pub fn test_parse3() {
    let example_score = r#"
e|---|
B|-3-|
G|6-6|
D|---|
A|---|
E|---|

e|---|
B|3-3|
G|-6-|
D|---|
A|---|
E|---|
"#;
    insta::assert_debug_snapshot!(parse3(
        &example_score
            .lines()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
    ));
}
