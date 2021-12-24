//! Extra utility functions specially used for the [`Registry`] part of this
//! crate

use chrono::{DateTime, Local};
use std::time::SystemTime;

/// Convert a [`SystemTime`](std::time::SystemTime) to
/// [`DateTime`](chrono::DateTime)
pub(crate) fn convert_to_datetime(t: SystemTime) -> DateTime<Local> {
    DateTime::<Local>::from(t)
}
