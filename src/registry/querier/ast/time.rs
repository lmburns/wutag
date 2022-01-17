//! Time-filter AST

use anyhow::{anyhow, Result};
use chrono::{offset::TimeZone, DateTime, Local, NaiveDate};
use std::{str::FromStr, time::SystemTime};

/// A filter to apply to file access, creation, and modification times
///
/// Credit: The idea and similar usage of this struct came from `sharkdp/fd`
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TimeFilter {
    /// A time happening **before**  another
    Before(SystemTime),
    /// A time happening **after**  another
    After(SystemTime),
    /// A time that **is** another
    Exact(SystemTime),
}

impl TimeFilter {
    /// Convert a string to a `TimeFilter`
    ///  - `tref` = The current time to compare the filter to
    ///  - `s` = The time before, after, or exact
    ///  - `exact` = If time is not a comparison & is instead an exact search
    fn from_str<S: AsRef<str>>(tref: &SystemTime, s: S, exact: bool) -> Result<SystemTime> {
        let str = s.as_ref();

        let time = exact
            .then(|| {
                let time: Option<SystemTime> =
                    humantime::Timestamp::from_str(str).ok().map(Into::into);
                time
            })
            .unwrap_or_else(|| humantime::parse_duration(str).map(|dur| *tref - dur).ok())
            .or_else(|| {
                DateTime::parse_from_rfc3339(str)
                    .map(Into::into)
                    .ok()
                    .or_else(|| {
                        NaiveDate::parse_from_str(str, "%F")
                            .map(|naive| naive.and_hms(0, 0, 0))
                            .ok()
                            .and_then(|dt| chrono::Local.from_local_datetime(&dt).single())
                    })
                    .or_else(|| chrono::Local.datetime_from_str(str, "%F %T").ok())
                    .map(Into::into)
            });

        time.ok_or_else(|| anyhow!("invalid time duration: {}", str))
    }

    /// Return a `TimeFilter` that is before a given time
    pub(crate) fn before<S: AsRef<str>>(tref: &SystemTime, s: S) -> Result<Self> {
        Self::from_str(tref, s, false).map(Self::Before)
    }

    /// Return a `TimeFilter` that is after a given time
    pub(crate) fn after<S: AsRef<str>>(tref: &SystemTime, s: S) -> Result<Self> {
        Self::from_str(tref, s, false).map(Self::After)
    }

    /// Return a `TimeFilter` that is to match a given time.
    ///
    /// Has to be an exact full datetime match like:
    ///  - `2022-01-01 00:01:01`
    ///  - `2018-02-16T00:31:37Z`
    pub(crate) fn exact<S: AsRef<str>>(tref: &SystemTime, s: S) -> Result<Self> {
        Self::from_str(tref, s, true).map(Self::Exact)
    }

    /// Determine whether the given time correctly applies to the filter
    ///   - `Before` matches if the given time is less than
    ///   - `After` matches if the given time is greater than
    ///   - `Exact` matches if the given time is
    pub(crate) fn does_match(&self, tref: &SystemTime) -> bool {
        match *self {
            Self::Before(ref max) => tref < max,
            Self::After(ref max) => tref > max,
            Self::Exact(ref is) => tref == is,
        }
    }
}
