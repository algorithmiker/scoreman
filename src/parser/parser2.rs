use anyhow::{bail, Context};
use std::io::BufRead;

use crate::collect_parse_error;

use super::{comment_line, partline, Score, Section};

pub fn parse2<A: BufRead>(inp: A) -> anyhow::Result<Score> {
    let mut sections = vec![];
    // Todo eventually remove Part
    let mut part_buf = vec![];
    let mut part_begin = 0;
    for (line_idx, line) in inp.lines().enumerate() {
        let line_number = line_idx + 1;
        let line = line.with_context(|| format!("Cannot read line {line_number}"))?;
        if line.trim().is_empty() {
            if !part_buf.is_empty() {
                println!("[W]: Empty line inside Part at line {line_idx}, are you sure this is intended?");
            }
            continue;
        }

        match comment_line(&line) {
            Ok((rem, comment)) => {
                if !rem.is_empty() {
                    bail!("Invalid comment syntax at line {line_number} (got remaining `{rem}`)")
                }
                if !part_buf.is_empty() {
                    println!("[W]: Comment inside Part at line {line_idx}, are you sure this is intended?");
                }
                sections.push(Section::Comment(comment.to_string()));
            }
            Err(_) => match partline(&line) {
                Ok((rem, mut line)) => {
                    if !rem.is_empty() {
                        bail!("Invalid partline at line {line_number}, got remaining content: `{rem}`");
                    }
                    // Add measure metadata
                    for measure_idx in 0..line.staffs.len() {
                        line.staffs[measure_idx].parent_line = Some(line_idx);
                        line.staffs[measure_idx].index_on_parent_line = Some(measure_idx);
                    }
                    part_buf.push(line);
                    match part_buf.len() {
                        6 => {
                            // flush part buf
                            sections.push(Section::Part {
                                part: part_buf
                                    .clone() // TODO try to elide this clone
                                    .try_into()
                                    .expect("Unreachable: more than 6 elements in part_buf"),
                                begin_line_idx: part_begin,
                                end_line_idx: line_idx,
                            });
                            part_buf.clear();
                            part_begin = 0;
                        }
                        1 => part_begin = line_idx,
                        _ => (),
                    }
                }
                // TODO maybe pass the error up here?
                Err(x) => bail!(
                    "Error at line {line_number}, not a comment but also not a valid partline\n{}",
                    collect_parse_error(x)
                ),
            },
        }
    }

    Ok(Score(sections))
}

#[test]
fn test_parse2() {
    use std::io::BufReader;
    let i1 = r#"
e|---|
B|-3-|
G|6-6|
D|---|
A|---|
E|---|

// This is a comment 

e|---|
B|3-3|
G|-6-|
D|---|
A|---|
E|---|"#;
    insta::assert_debug_snapshot!(parse2(BufReader::new(i1.as_bytes())));
    let i2 = r#"
e|---|
B|-3-|
G|6-6|
D|---|
A|-o-|
E|---|

// This is a comment 

e|---|
B|3-3|
G|-6-|
D|---|
A|---|
E|---|"#;
    insta::assert_debug_snapshot!(parse2(BufReader::new(i2.as_bytes())));
}
