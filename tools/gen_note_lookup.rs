fn main() {
    let mut lookup = [None; 256];
    let mut set_lookup = |c: char, value: u8| {
        lookup[c as usize] = Some(value);
    };

    set_lookup('E', 3 * 12 + 4);
    set_lookup('A', 3 * 12 + 9);
    set_lookup('D', 4 * 12 + 2);
    set_lookup('G', 4 * 12 + 7);
    set_lookup('B', 4 * 12 + 1);
    set_lookup('d', 5 * 12 + 2);
    set_lookup('e', 5 * 12 + 4);

    println!(
        "#[rustfmt::skip]\nconst STRING_BASE_NOTES: [Option<u8>;{}] ={lookup:?};",
        lookup.len()
    );
}
