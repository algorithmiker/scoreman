use std::io::BufReader;

use crate::{
    backend::{
        muxml2::{settings::Settings, Muxml2Backend},
        Backend,
    },
    parser::parser2::parse2,
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
    };
    Muxml2Backend::process(parse2(BufReader::new(i1.as_bytes()))?, &mut out, settings)?;
    insta::assert_snapshot!(String::from_utf8_lossy(&out));
    Ok(())
}
