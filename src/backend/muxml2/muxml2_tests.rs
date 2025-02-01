use crate::backend::errors::backend_error_kind::BackendErrorKind;
use crate::backend::{
    muxml2::{settings::Settings, Muxml2Backend},
    Backend,
};
use crate::debugln;
use crate::parser::parser3::{parse3, source_location_from_stream};
use itertools::Itertools;

#[test]
fn test_muxml2() -> anyhow::Result<()> {
    let i1 = r#"
e|--------------|
B|-----------0-1|
G|-------1-2----|
D|--0-2-4-------|
A|-3------------|
E|--------------|
    "#;
    let mut out = vec![];
    let settings = Settings {
        remove_rest_between_notes: true,
        trim_measure: true,
        simplify_time_signature: true,
        bend_mode: crate::backend::muxml2::settings::Muxml2BendMode::EmulateBends,
    };
    Muxml2Backend::process(
        &i1.lines().map(|x| x.to_string()).collect::<Vec<_>>(),
        &mut out,
        settings,
    );
    insta::assert_snapshot!(String::from_utf8_lossy(&out));
    Ok(())
}
#[test]
fn test_muxml_bends() -> anyhow::Result<()> {
    let i1 = r#"
e|----|
B|----|
G|2b--|
D|----|
A|----|
E|----|
    "#;
    let mut out = vec![];
    let settings = Settings {
        remove_rest_between_notes: true,
        trim_measure: true,
        simplify_time_signature: true,
        bend_mode: crate::backend::muxml2::settings::Muxml2BendMode::EmulateBends,
    };
    Muxml2Backend::process(
        &i1.lines().map(|x| x.to_string()).collect::<Vec<_>>(),
        &mut out,
        settings,
    );
    insta::assert_snapshot!(String::from_utf8_lossy(&out));
    Ok(())
}

#[test]
pub fn test_invalid_bends() {
    let example_score = r#"
e|--12-12|--12-12|--12-12-|
B|3------|3------|3----11-|
G|-6-3-3-|-6-3-3-|-6-3---b|
D|-------|-------|-----11-|
A|-------|-------|-----11-|
E|-----9-|-----9-|-----11-|

// This is a comment!

e|--12-12|--12-12|
B|3------|3------|
G|-6-3-3-|-6-3-3-|
D|-------|-------|
A|-------|-------|
E|-----9-|-----9-|"#;
    let settings = Settings {
        remove_rest_between_notes: true,
        trim_measure: true,
        simplify_time_signature: true,
        bend_mode: crate::backend::muxml2::settings::Muxml2BendMode::EmulateBends,
    };
    let res = Muxml2Backend::process(
        &example_score.lines().map(|x| x.to_string()).collect_vec(),
        &mut Vec::new(),
        settings,
    );
    let e = res.err.unwrap();
    use crate::backend::errors::error_location::ErrorLocation;
    assert_eq!(e.main_location, ErrorLocation::LineAndChar(3, 25));
    assert!(matches!(e.kind, BackendErrorKind::BendOnInvalid));
}
