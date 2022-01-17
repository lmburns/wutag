//! Extra utility functions specially used for the [`Registry`] part of this
//! crate

use crate::regex;
use chrono::{DateTime, Local};
use regex::{Captures, Regex};
use std::{borrow::Cow, time::SystemTime};

/// Convert a [`SystemTime`](std::time::SystemTime) to
/// [`DateTime`](chrono::DateTime)
pub(crate) fn convert_to_datetime(t: SystemTime) -> DateTime<Local> {
    DateTime::<Local>::from(t)
}

/// Unescape an octal sequence
pub(crate) fn unescape_octal(s: &str) -> Cow<'_, str> {
    let reg = regex!(r"([0-7]{3})");
    let unescape = |s: &str| u32::from_str_radix(s, 8).expect("failed to parse to octal");

    reg.replace_all(s, |caps: &Captures| {
        unescape(caps.get(0).map_or("", |m| m.as_str())).to_string()
    })
}
