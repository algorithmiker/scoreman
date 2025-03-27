use crate::parser::numeric;
use std::cmp::max;

#[derive(Debug, PartialEq, Clone)]
pub enum TabElement {
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

#[derive(Debug)]
pub enum TabElementError {
    FretTooLarge,
}
#[inline(always)]
pub fn tab_element3(s: &str) -> Result<(&str, TabElement), (&str, Option<TabElementError>)> {
    let bytes = s.as_bytes();
    match bytes.first() {
        Some(b'-') => Ok((&s[1..], TabElement::Rest)),
        Some(b'x') => Ok((&s[1..], TabElement::DeadNote)),
        Some(48..=58) => {
            let (res, num) = numeric(s).map_err(|s| (s, Some(TabElementError::FretTooLarge)))?;
            Ok((res, TabElement::Fret(num)))
        }
        Some(b'b') => Ok((&s[1..], TabElement::Bend)),
        Some(b'h') => Ok((&s[1..], TabElement::HammerOn)),
        Some(b'p') => Ok((&s[1..], TabElement::Pull)),
        Some(b'r') => Ok((&s[1..], TabElement::Release)),
        Some(b'/') | Some(b'\\') => Ok((&s[1..], TabElement::Slide)),
        Some(b'~') => Ok((&s[1..], TabElement::Vibrato)),
        Some(_) | None => Err((s, None)),
    }
}
impl TabElement {
    pub fn repr_len(&self) -> u32 {
        use TabElement::*;
        match self {
            Fret(x) => max(x, &1).ilog10() + 1,
            Bend | HammerOn | DeadNote | Pull | Slide | Rest | Release | Vibrato => 1,
        }
    }
}
