//! Example usage (as a library):
//! ```
//! use scoreman::backend::BackendSelector;
//! use scoreman::BufLines;
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
//! my_backend.process(&input.into(), &mut out);
//!```
use std::{
    ops::{Range, RangeInclusive},
    time::{Duration, Instant},
};

use memchr::memchr_iter;

pub mod backend;
pub mod parser;

#[derive(Clone)]
/// A buffer that holds lines. Like a Vec<String>, but allocates the data only once.
pub struct BufLines {
    pub buf: String,
    pub line_ends: Vec<usize>,
}
impl BufLines {
    /// ```rust
    /// use scoreman::BufLines;
    /// let buf = String::from("Hello\nBeautiful\nWorld");
    /// let lines = BufLines::from_string(buf);
    /// assert_eq!(lines.get_line(0), "Hello");
    /// assert_eq!(lines.get_line(0).len(), lines.line_len(0));
    /// assert_eq!(lines.get_line(1), "Beautiful");
    /// assert_eq!(lines.get_line(1).len(), lines.line_len(1));
    /// assert_eq!(lines.get_line(2), "World");
    /// ```
    pub fn from_string(buf: String) -> Self {
        let mut line_ends = Vec::with_capacity(buf.len() / 32);
        line_ends.extend(memchr_iter(b'\n', buf.as_bytes()));
        line_ends.push(buf.len());
        Self { buf, line_ends }
    }

    pub fn line_count(&self) -> usize {
        self.line_ends.len()
    }
    pub fn line_len(&self, line_idx: usize) -> usize {
        self.line_byte_range(line_idx).len()
    }
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        (0..self.line_count()).map(|x| &self.buf[self.line_byte_range(x)])
    }
    pub fn line_byte_range(&self, line_idx: usize) -> Range<usize> {
        let start = if line_idx == 0 { 0 } else { self.line_ends[line_idx - 1] + 1 };
        start..self.line_ends[line_idx]
    }
    pub fn line_byte_range_checked(&self, line_idx: usize) -> Option<Range<usize>> {
        let start = self.line_ends.get(line_idx)?;
        let end = self.line_ends.get(line_idx + 1)?;
        Some(*start..*end)
    }
    pub fn get_line(&self, idx: usize) -> &str {
        &self.buf[self.line_byte_range(idx)]
    }
    pub fn get_line_mut(&mut self, idx: usize) -> &mut str {
        let range = self.line_byte_range(idx);
        &mut self.buf[range]
    }
    pub fn get_line_checked(&self, idx: usize) -> Option<&str> {
        let range = self.line_byte_range_checked(idx)?;
        Some(&self.buf[range.clone()])
    }
}
impl From<String> for BufLines {
    fn from(value: String) -> Self {
        Self::from_string(value)
    }
}
impl<'a> From<&'a str> for BufLines {
    fn from(value: &'a str) -> Self {
        Self::from_string(value.into())
    }
}
impl From<BufLines> for String {
    fn from(val: BufLines) -> Self {
        val.buf
    }
}
/// A thing that the parser can use as a line.
pub trait ParseLines {
    fn get_line(&self, idx: usize) -> &str;
    fn line_count(&self) -> usize;
}
impl<T: AsRef<str>> ParseLines for Vec<T> {
    fn get_line(&self, idx: usize) -> &str {
        self[idx].as_ref()
    }

    fn line_count(&self) -> usize {
        self.len()
    }
}
impl ParseLines for BufLines {
    fn get_line(&self, idx: usize) -> &str {
        self.get_line(idx)
    }

    fn line_count(&self) -> usize {
        self.line_count()
    }
}
pub fn time_print<T>(tag: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let r = f();
    println!("{tag} took {:?}", start.elapsed());
    r
}
pub fn rlen<T: std::ops::Sub<Output = T> + Copy + std::ops::Add<u32, Output = T>>(
    r: &RangeInclusive<T>,
) -> T {
    *r.end() - *r.start() + 1
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
