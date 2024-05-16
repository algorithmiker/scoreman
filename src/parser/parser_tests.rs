use crate::parser::{measure, parser2::parse2, partline, TabElement};

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
    insta::assert_debug_snapshot!(partline("e|--4-|-0--5-|"));
}

#[test]
fn test_measure() {
    use TabElement::*;
    fn test(s: &str, frets: &[TabElement]) {
        match measure(s) {
            Ok((remaining, actual_frets)) => {
                assert_eq!(actual_frets.content.as_slice(), frets);
                assert_eq!(remaining, "");
            }

            Err(x) => panic!("Got error when testing {s}: {x}"),
        }
    }
    test("|--4-", &[Rest, Rest, Fret(4), Rest]);
}
