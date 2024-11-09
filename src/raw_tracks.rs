use crate::parser::{Measure, RawTick, TabElement};

pub type RawTracks = ([char; 6], [Vec<Measure>; 6]);

pub fn find_multichar_tick(
    tracks: &[Vec<Measure>; 6],
    measure_idx: usize,
    track_names: [char; 6],
    tick_idx: usize,
) -> std::option::Option<(usize, &RawTick)> {
    tracks
        .iter()
        .enumerate()
        .map(|(t_idx, track)| {
            (
                t_idx,
                track[measure_idx].content.get(tick_idx).unwrap_or_else(|| {
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
fn test_multichar_raw_tracks() -> anyhow::Result<()> {
    use crate::parser::parser2::parse2;
    let input = r#"
e|----5--|
B|---3---|
G|10---12|
D|-------|
A|-------|
E|-------|"#;
    insta::assert_debug_snapshot!(parse2(input.lines()));
    Ok(())
}
