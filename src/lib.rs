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
//! my_backend.process(input.map(|x|x.into()).collect(), &mut out);
//!```
use std::{
    ops::{Range, RangeInclusive},
    time::{Duration, Instant},
};

pub mod backend;
pub mod parser;
#[macro_export]
macro_rules! traceln {
    (depth=$depth:literal, $($t:expr),*) => {
        #[cfg(feature="gt_trace")]
        {
            use yansi::Paint;
            let padding=" ".repeat($depth);
            println!("{padding}{} {}", "[T]:".blue().bold(), format_args!($($t),*));
        }
    };
    ($($t:expr),*) => {
        #[cfg(feature="gt_trace")]
        {
            use yansi::Paint;
            println!("{} {}", "[T]:".blue().bold(), format_args!($($t),*));
        }
    };
}
#[macro_export]
macro_rules! debugln {
    (depth=$depth:literal, $($t:expr),*) => {
        #[cfg(feature="gt_debug")]
        {
            use yansi::Paint;
            let padding=" ".repeat($depth);
            println!("{padding}{} {}", "[D]:".green().bold(), format_args!($($t),*));
        }
    };
    ($($t:expr),*) => {
        #[cfg(feature="gt_debug")]
        {
            use yansi::Paint;
            println!("{} {}", "[D]:".green().bold(), format_args!($($t),*));
        }
    };
}

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
