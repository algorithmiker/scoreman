use std::collections::HashMap;

use crate::parser::{parser2::Parser2, partline};

#[test]
fn test_score() {
    let parser = Parser2::default();
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
    insta::assert_debug_snapshot!(parser.parse(example_score.lines()));
}
#[test]
fn test_part() {
    let example_part = r#"
e|---|---|
B|-3-|3-3|
G|6-6|-6-|
D|---|---|
A|---|---|
E|---|---|"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(example_part.lines()));
}

#[test]
fn test_partline() {
    let mut string_buf = vec![];
    let mut string_measure_buf = vec![];
    let mut offsets = vec![];
    let mut bend_targets = HashMap::new();
    partline(
        "e|--4-|-0--5-|",
        0,
        0,
        &mut string_buf,
        &mut string_measure_buf,
        &mut offsets,
        &mut bend_targets,
        0,
        true,
    )
    .unwrap();
    insta::assert_debug_snapshot!((string_buf, offsets));
}

#[test]
fn test_bend() {
    let mut string_buf = vec![];
    let mut string_measure_buf = vec![];
    let mut offsets = vec![];
    let mut bend_targets = HashMap::new();
    partline(
        "e|--4b-|-0--5-|",
        0,
        0,
        &mut string_buf,
        &mut string_measure_buf,
        &mut offsets,
        &mut bend_targets,
        0,
        true,
    )
    .unwrap();
    insta::assert_debug_snapshot!((string_buf, offsets));

    let mut bend_targets = HashMap::new();
    let mut string_buf = vec![];
    let mut string_measure_buf = vec![];
    let mut offsets = vec![];
    partline(
        "e|--4b|-0--5-|",
        0,
        0,
        &mut string_buf,
        &mut string_measure_buf,
        &mut offsets,
        &mut bend_targets,
        0,
        true,
    )
    .unwrap();
    insta::assert_debug_snapshot!((string_buf, offsets));
}
#[test]
fn test_bend_to() {
    let mut string_buf = vec![];
    let mut string_measure_buf = vec![];
    let mut offsets = vec![];
    let mut bend_targets = HashMap::new();
    partline(
        "e|--4b5-|-0--5-|",
        0,
        0,
        &mut string_buf,
        &mut string_measure_buf,
        &mut offsets,
        &mut bend_targets,
        0,
        true,
    )
    .unwrap();
    insta::assert_debug_snapshot!((string_buf, offsets));

    // invalid
    let mut string_buf = vec![];
    let mut string_measure_buf = vec![];
    let mut offsets = vec![];
    let mut bend_targets = HashMap::new();
    let err = partline(
        "e|--b5-|-0--5-|",
        0,
        0,
        &mut string_buf,
        &mut string_measure_buf,
        &mut offsets,
        &mut bend_targets,
        0,
        true,
    );
    insta::assert_debug_snapshot!(err);
}

#[test]
fn test_left_bend_score() {
    let score = r#"
e|--12b-|
B|--3--0|
G|2-----|
D|------|
A|------|
E|------|
"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(score.lines()));
    let score = r#"
e|--12b-|
B|--3--4|
G|2-----|
D|------|
A|------|
E|------|
"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(score.lines()));
}

#[test]
fn test_right_bend_score() {
    let score = r#"
e|--12b|
B|-0--3|
G|2----|
D|-----|
A|-----|
E|-----|
"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(score.lines()));
}

#[test]
fn test_full_bend_score() {
    let score = r#"
e|--12b|
B|--12b|
G|2----|
D|-----|
A|-----|
E|-----|
"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(score.lines()));
    let score = r#"
e|--12b|
B|--12b|
G|2-12b|
D|--12b|
A|--12b|
E|--12b|
"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(score.lines()));
}

#[test]
fn test_bendy_score() {
    let example_part = r#"
e|--12b---12b-|
B|--3---0---3-|
G|2-----------|
D|------------|
A|------------|
E|------------|
"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(example_part.lines()));
    let score = r#"
e|--12b-12b-|
B|--3---12b-|
G|2-----12b-|
D|------12b-|
A|------12b-|
E|------12b-|
"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(score.lines()));
}
