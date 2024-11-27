use crate::backend::{
    muxml2::{settings::Settings, Muxml2Backend},
    Backend,
};

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
    Muxml2Backend::process(i1.lines(), &mut out, settings);
    insta::assert_snapshot!(String::from_utf8_lossy(&out));
    Ok(())
}
#[test]
fn test_muxml_bends() -> anyhow::Result<()> {
    let i1 = r#"
e|----|
B|----|
G|2b4-|
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
    Muxml2Backend::process(i1.lines(), &mut out, settings);
    insta::assert_snapshot!(String::from_utf8_lossy(&out));
    Ok(())
}
