use crate::{
    parser::{dump_tracks, Parser},
    BufLines,
};

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
    let parsed = Parser::parse(&BufLines::from_string(example_score.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
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

    let parsed = Parser::parse(&BufLines::from_string(example_part.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
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
    let parsed = Parser::parse(&BufLines::from_string(score.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
    let score = r#"
e|--12b-|
B|--3--4|
G|2-----|
D|------|
A|------|
E|------|
"#;
    let parsed = Parser::parse(&BufLines::from_string(score.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
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
    let parsed = Parser::parse(&BufLines::from_string(score.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
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
    let parsed = Parser::parse(&BufLines::from_string(score.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
    let score = r#"
e|--12b|
B|--12b|
G|2-12b|
D|--12b|
A|--12b|
E|--12b|
"#;
    let parsed = Parser::parse(&BufLines::from_string(score.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
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
    let parsed = Parser::parse(&BufLines::from_string(example_part.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
    let score = r#"
e|--12b-12b-|
B|--3---12b-|
G|2-----12b-|
D|------12b-|
A|------12b-|
E|------12b-|
"#;
    let parsed = Parser::parse(&BufLines::from_string(score.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
}
#[test]
fn test_multichar_tracks() -> anyhow::Result<()> {
    let input = r#"
e|----5--|
B|---3---|
G|10---12|
D|-------|
A|-------|
E|-------|"#;

    let parsed = Parser::parse(&BufLines::from_string(input.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
    Ok(())
}

#[test]
pub fn test_parse3() {
    use std::time::Instant;
    let example_score = r#"
e|--12-12|--12-12|--12-12|
B|3------|3------|3----11|
G|-6-3-3-|-6-3-3-|-6-3-11|
D|-------|-------|-----11|
A|-------|-------|-----11|
E|-----9-|-----9-|-----11|

// This is a comment!

e|--12-12|--12-12|
B|3------|3------|
G|-6-3-3-|-6-3-3-|
D|-------|-------|
A|-------|-------|
E|-----9-|-----9-|
"#;
    let time_parser3 = Instant::now();
    let parsed = Parser::parse(&BufLines::from_string(example_score.into())).unwrap();
    insta::assert_snapshot!(dump_tracks(&parsed.as_ref()));
    println!("Parser3 took: {:?}", time_parser3.elapsed());
    insta::assert_debug_snapshot!(parsed);
}
