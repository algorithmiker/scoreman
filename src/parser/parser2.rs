use crate::backend::errors::{
    backend_error::BackendError, backend_error_kind::BackendErrorKind, diagnostic::Diagnostic,
    error_location::ErrorLocation,
};

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
        if line.trim().is_empty() {
            if !part_buf.is_empty() {
                diagnostics.push(Diagnostic::warn(
                    ErrorLocation::LineOnly(line_idx),
                    "Empty line inside Part, are you sure this is intended?".into(),
                ));
            }
            continue;
        }

        if let Ok((rem, comment)) = comment_line(line) {
            assert!(rem.is_empty(), "Invalid comment syntax (line {line_idx})");
            if !part_buf.is_empty() {
                diagnostics.push(Diagnostic::warn(
                    ErrorLocation::LineOnly(line_idx),
                    "Comment inside Part, are you sure this is intended?".to_string(),
                ));
            }
            sections.push(Section::Comment(comment.to_string()));
        } else {
            match partline(line, line_idx) {
                Ok((rem, line)) => {
                    if !rem.is_empty() {
                        return Err(BackendError {
                            main_location: ErrorLocation::LineAndMeasure(
                                line_idx,
                                // the measure with the problem is the first that is not parsed
                                line.measures.len(),
                            ),
                            relevant_lines: line_idx..=line_idx,
                            kind: BackendErrorKind::InvalidPartlineSyntax(rem.into()),
                            diagnostics,
                        });
                    }
                    part_buf.push(line);
                    if part_buf.len() == 6 {
                        // flush part buf
                        sections.push(Section::Part {
                            part: part_buf
                                .try_into()
                                .expect("Unreachable: more than 6 elements in part_buf"),
                            begin_line_idx: part_begin,
                            end_line_idx: line_idx,
                        });
                        part_buf = vec![];
                        part_begin = 0;
                    } else if part_buf.len() == 1 {
                        part_begin = line_idx
                    }
                }
                Err(x) => {
                    return Err(BackendError {
                        main_location: ErrorLocation::LineOnly(line_idx),
                        relevant_lines: line_idx..=line_idx,
                        kind: BackendErrorKind::ParseError(x),
                        diagnostics,
                    });
                }
            }
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
