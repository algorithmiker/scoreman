use crate::parser::{parser2::parse2, partline};

#[test]
fn test_score() {
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
    insta::assert_debug_snapshot!(parse2(example_score.lines()));
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
    insta::assert_debug_snapshot!(parse2(example_part.lines()));
}

#[test]
fn test_partline() {
    insta::assert_debug_snapshot!(partline("e|--4-|-0--5-|", 0));
}
