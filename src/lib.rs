//! Example usage (as a library):
//! ```
//! use scoreman::backend::BackendSelector;
//! let input = r#"
//! e|---|
//! A|---|
//! B|---|
//! G|---|
//! D|---|
//! E|---|
//! "#;
//! let my_backend = BackendSelector::Midi;
//! let mut out = vec![];
//! my_backend.process(&input.lines().map(|x|x.into()).collect::<Vec<_>>(), &mut out);
//!```
use std::{
    ops::{Range, RangeInclusive},
    time::{Duration, Instant},
};

pub mod backend;
pub mod parser;

pub fn rlen<T: std::ops::Sub<Output = T> + Copy + std::ops::Add<u32, Output = T>>(
    r: &RangeInclusive<T>,
) -> T {
    *r.end() - *r.start() + 1
}

pub fn rcontains<
    T: std::ops::Sub<Output = T> + Copy + std::ops::Add<usize, Output = T> + PartialOrd<T>,
>(
    r: &Range<T>, elem: T,
) -> bool {
    elem >= r.start && elem < r.end
}

pub fn ricontains<
    T: std::ops::Sub<Output = T> + Copy + std::ops::Add<usize, Output = T> + PartialOrd<T>,
>(
    r: &RangeInclusive<T>, elem: T,
) -> bool {
    elem >= *r.start() && elem <= *r.end()
}

pub fn time<T, F: FnOnce() -> T>(f: F) -> (Duration, T) {
    let start = Instant::now();
    let val = f();
    (start.elapsed(), val)
}
pub fn digit_cnt_usize(num: usize) -> u32 {
    num.checked_ilog10().unwrap_or(0) + 1
}
pub fn digit_cnt_u8(num: u8) -> u8 {
    // if num > 99 {
    //     3
    // } else if num > 9 {
    //     2
    // } else {
    //     1
    // }
    num.checked_ilog10().unwrap_or(0) as u8 + 1
}
