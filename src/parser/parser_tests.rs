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
    partline(
        "e|--4-|-0--5-|",
        0,
        0,
        &mut string_buf,
        &mut string_measure_buf,
        &mut offsets,
    )
    .unwrap();
    insta::assert_debug_snapshot!((string_buf, offsets));
}
