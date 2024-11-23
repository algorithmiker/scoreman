use crate::parser::{Measure, RawTick, TabElement};
/// TODO: merge this file into parser.mod
pub fn find_multichar_tick<'a>(
    strings: &'a [Vec<RawTick>; 6],
    measures: &[Vec<Measure>; 6],
    measure_idx: usize,
    track_names: &[char; 6],
    tick_idx: usize,
) -> std::option::Option<(usize, &'a RawTick)> {
    strings
        .iter()
        .enumerate()
        .map(|(t_idx, track)| {
            (
                t_idx,
                measures[t_idx][measure_idx]
                    .get_content(track)
                    .get(tick_idx)
                    .unwrap_or_else(|| {
                        panic!(
                            "Measure {} on string {} doesn't have tick {t_idx}\n",
                            measure_idx + 1,
                            track_names[t_idx]
                        );
                    }),
            )
        })
        .find(|(_, x)| match x.element {
            TabElement::Fret(x) => x >= 10,
            _ => false,
        })
}
#[test]
fn test_multichar_tracks() -> anyhow::Result<()> {
    use crate::parser::parser2::Parser2;
    let input = r#"
e|----5--|
B|---3---|
G|10---12|
D|-------|
A|-------|
E|-------|"#;
    insta::assert_debug_snapshot!(Parser2::default().parse(input.lines()));
    Ok(())
}
