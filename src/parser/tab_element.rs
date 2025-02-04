use crate::parser::numeric;
use std::cmp::max;

#[derive(Debug, PartialEq, Clone)]
pub enum TabElement3 {
    Fret(u8),
    Rest,
    DeadNote,
    Bend,
    HammerOn,
    Pull,
    Release,
    Slide,
    Vibrato,
}

#[inline(always)]
pub fn tab_element3(s: &str) -> Result<(&str, TabElement3), &str> {
    let bytes = s.as_bytes();
    match bytes.first() {
        Some(b'-') => Ok((&s[1..], TabElement3::Rest)),
        Some(b'x') => Ok((&s[1..], TabElement3::DeadNote)),
        Some(48..=58) => {
            let (res, num) = numeric(s)?;
            Ok((res, TabElement3::Fret(num)))
        }
        Some(b'b') => Ok((&s[1..], TabElement3::Bend)),
        Some(b'h') => Ok((&s[1..], TabElement3::HammerOn)),
        Some(b'p') => Ok((&s[1..], TabElement3::Pull)),
        Some(b'r') => Ok((&s[1..], TabElement3::Release)),
        Some(b'/') | Some(b'\\') => Ok((&s[1..], TabElement3::Slide)),
        Some(b'~') => Ok((&s[1..], TabElement3::Vibrato)),
        Some(_) | None => Err(s),
    }
}
impl TabElement3 {
    pub fn repr_len(&self) -> u32 {
        use TabElement3::*;
        match self {
            Fret(x) => max(x, &1).ilog10() + 1,
            Bend | HammerOn | DeadNote | Pull | Slide | Rest | Release | Vibrato => 1,
        }
    }
}
