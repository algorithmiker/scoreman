use std::{
    ops::RangeInclusive,
    time::{Duration, Instant},
};

pub mod backend;
#[cfg(test)]
mod fs_test;
pub mod parser;
pub mod raw_tracks;

// FIXME: I think a bunch of things have off-by-one errors because of this
pub fn rlen<T: std::ops::Sub<Output = T> + Copy + std::ops::Add<usize, Output = T>>(
    r: &RangeInclusive<T>,
) -> T {
    return *r.end() - *r.start() + 1;
}

pub fn time<T, F: FnOnce() -> T>(f: F) -> (Duration, T) {
    let start = Instant::now();
    let val = f();
    (start.elapsed(), val)
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
