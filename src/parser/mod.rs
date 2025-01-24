pub mod parser2;
pub mod parser3;
#[cfg(test)]
mod parser_tests;

use crate::{digit_cnt_u8, rlen};
use std::ops::RangeInclusive;

pub fn char(c: char) -> impl Fn(&str) -> Result<(&str, char), &str> {
    move |s: &str| match s.chars().next() {
        Some(cc) if cc == c => Ok((&s[1..], c)),
        _ => Err(s),
    }
}

pub fn string_name() -> impl Fn(&str) -> Result<(&str, char), &str> {
    move |s: &str| match s.chars().next() {
        Some(c) if c.is_alphabetic() => Ok((&s[1..], c)),
        _ => Err(s),
    }
}

#[inline(always)]
fn numeric(s: &str) -> Result<(&str, u8), &str> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut sum = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        sum *= 10;
        sum += bytes[i] - b'0';
        i += 1;
    }
    if i == 0 {
        return Err(s);
    };
    Ok((&s[i..], sum))
}
