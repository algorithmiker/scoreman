use std::fmt::Write;
use std::num::NonZeroU8;
fn main() {
    let mut lookup = vec![String::from("None"); 256];
    let mut set_lookup = |c: char, value: u8| {
        lookup[c as usize] = format!("Some(NonZeroU8::new({value}).unwrap_unchecked())");
    };
    // octave * 12 + offset in the 12-scale
    // (so octave 0, C = 0)
    set_lookup('E', 3 * 12 + 4);
    set_lookup('A', 3 * 12 + 9);
    set_lookup('D', 4 * 12 + 2);
    set_lookup('G', 4 * 12 + 7);
    set_lookup('B', 4 * 12 + 11);
    set_lookup('d', 5 * 12 + 2);
    set_lookup('e', 5 * 12 + 4);

    let mut b = String::from("[");
    for x in &lookup {
        b.push_str(&x.to_string());
        b.push_str(", ");
    }
    b.pop();
    b.pop();
    b.push(']');
    println!(
        "#[rustfmt::skip]\nconst STRING_BASE_NOTES: [Option<NonZeroU8>;{}] = unsafe {{{b}}};",
        lookup.len()
    );
}
