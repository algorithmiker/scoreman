use crate::backend::errors::{BackendError, BackendErrorKind, Diagnostic};

use super::{comment_line, partline, Score, Section};

pub fn parse2<'a, A: std::iter::Iterator<Item = &'a str>>(
    lines: A,
) -> Result<(Vec<Diagnostic>, Score), BackendError<'a>> {
    let mut diagnostics = vec![];
    let mut sections = vec![];
    // Todo eventually remove Part
    let mut part_buf = vec![];
    let mut part_begin = 0;
    for (line_idx, line) in lines.enumerate() {
        let line_number = line_idx + 1;
        if line.trim().is_empty() {
            if !part_buf.is_empty() {
                diagnostics.push(Diagnostic::warn(
                    Some((line_idx, 0)),
                    "Empty line inside Part, are you sure this is intended?".into(),
                ));
            }
            continue;
        }

        match comment_line(line) {
            Ok((rem, comment)) => {
                if !rem.is_empty() {
                    return Err(BackendError {
                        main_location: Some((line_number, 0)),
                        relevant_lines: line_number..=line_number,
                        kind: BackendErrorKind::InvalidCommentSyntax(rem.into()),
                        diagnostics,
                    });
                }
                if !part_buf.is_empty() {
                    diagnostics.push(Diagnostic::warn(
                        Some((line_idx, 0)),
                        "Comment inside Part at line {line_idx}, are you sure this is intended?"
                            .into(),
                    ));
                }
                sections.push(Section::Comment(comment.to_string()));
            }
            Err(_) => match partline(line) {
                Ok((rem, mut line)) => {
                    if !rem.is_empty() {
                        return Err(BackendError {
                            main_location: Some((line_number, 0)),
                            relevant_lines: line_number..=line_number,
                            kind: BackendErrorKind::InvalidPartlineSyntax(rem.into()),
                            diagnostics,
                        });
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
                Err(x) => {
                    return Err(BackendError {
                        main_location: Some((line_number, 0)),
                        relevant_lines: line_number..=line_number,
                        kind: BackendErrorKind::ParseError(x),
                        diagnostics,
                    });
                }
            },
        }
    }

    Ok((diagnostics, Score(sections)))
}

#[test]
fn test_parse2() {
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
    insta::assert_debug_snapshot!(parse2(i1.lines()));
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
    insta::assert_debug_snapshot!(parse2(i2.lines()));
}
