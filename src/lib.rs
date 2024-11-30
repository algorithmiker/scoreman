use std::{
    ops::{Range, RangeInclusive},
    time::{Duration, Instant},
};

pub mod backend;
#[cfg(test)]
mod fs_test;
pub mod parser;
pub mod raw_tracks;
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

pub fn rlen<T: std::ops::Sub<Output = T> + Copy + std::ops::Add<usize, Output = T>>(
    r: &RangeInclusive<T>,
) -> T {
    return *r.end() - *r.start() + 1;
}
pub fn rcontains<
    T: std::ops::Sub<Output = T> + Copy + std::ops::Add<usize, Output = T> + std::cmp::PartialOrd<T>,
>(
    r: &Range<T>,
    elem: T,
) -> bool {
    elem >= r.start && elem < r.end
}
pub fn ricontains<
    T: std::ops::Sub<Output = T> + Copy + std::ops::Add<usize, Output = T> + std::cmp::PartialOrd<T>,
>(
    r: &RangeInclusive<T>,
    elem: T,
) -> bool {
    return elem >= *r.start() && elem <= *r.end();
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

pub fn collect_parse_error(x: &nom::Err<VerboseError<&str>>) -> String {
    let mut collected = String::new();
    collected += &format!(
        "{}",
        match x {
            nom::Err::Incomplete(_) => unreachable!(),
            nom::Err::Error(x) => x,
            nom::Err::Failure(x) => x,
        }
    );
    collected
}

pub const BOLD_YELLOW_FORMAT: &str = "\x1b[1;33m";
pub const GREEN_FORMAT: &str = "\x1b[32m";
pub const CLEAR_FORMAT: &str = "\x1b[0m";

use nom::error::VerboseError;
pub use nom::Err;
use yansi::Paint;
