//! Versioning for the crate and database

#![allow(unused)]

use anyhow::{Context, Result};
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
};
use std::str::FromStr;
use thiserror::Error;

// ===================== Version ======================

/// Crate version -- used for making sure the user's database schema is up to
/// date and will be compatible with the current version of `wutag`
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    /// Major `Version`
    major: usize,
    /// Minor `Version`
    minor: usize,
    /// Patch `Version`
    patch: usize,
}

impl Version {
    /// Create a `Version` from specified values. Used for creating a `Version`
    /// from a database query
    pub(crate) const fn new(major: usize, minor: usize, patch: usize) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Create the `Version` struct from a string
    fn build_<S: AsRef<str>>(s: S) -> Result<Self> {
        let split = s
            .as_ref()
            .split('.')
            .map(|v| v.parse::<usize>().expect("failed to parse version"))
            .collect::<Vec<_>>();

        Ok(Self {
            major: *(split.get(0).context("failed to get major")?),
            minor: *(split.get(1).context("failed to get minor")?),
            patch: *(split.get(2).context("failed to get patch")?),
        })
    }

    /// Create the `Version` struct from the crate's version
    pub(crate) fn build() -> Result<Self> {
        Self::build_(&Self::string())
    }

    /// Return the crate version as a `String`
    pub(crate) fn string() -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    /// Return the already built version as a `String`
    pub(crate) fn as_str(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Return the `major` version
    pub(crate) const fn major(&self) -> usize {
        self.major
    }

    /// Return the `minor` version
    pub(crate) const fn minor(&self) -> usize {
        self.minor
    }

    /// Return the `patch` version
    pub(crate) const fn patch(&self) -> usize {
        self.patch
    }

    /// Determine if the `Version` is less than another
    pub(crate) const fn less_than(&self, other: Self) -> bool {
        // self < &other
        (self.major < other.major)
            || (self.major == other.major && self.minor < other.minor)
            || (self.major == other.major && self.minor == other.minor && self.patch < other.patch)
    }

    /// Determine if the `Version` is greater than another
    pub(crate) const fn greater_than(&self, other: Self) -> bool {
        // self > &other
        (self.major > other.major)
            || (self.major == other.major && self.minor > other.minor)
            || (self.major == other.major && self.minor == other.minor && self.patch > other.patch)
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::build_(s).map_err(|_| Error::FromString)
    }
}

impl ToSql for Version {
    fn to_sql(&self) -> rsq::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(Self::string()))
    }
}

impl FromSql for Version {
    fn column_result(val: ValueRef) -> FromSqlResult<Self> {
        match Self::from_str(val.as_str().expect("failed to convert Version to `str`")) {
            Ok(v) => Ok(v),
            Err(err) => Err(FromSqlError::Other(Box::new(err))),
        }
    }
}

impl From<Version> for ToSqlOutput<'_> {
    #[inline]
    fn from(t: Version) -> Self {
        ToSqlOutput::Owned(t.as_str().into())
    }
}

/// Error used to satisfy an `Error` with an `impl` of `std::error::Error` for
/// the `FromSql` `impl`
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// Unable to convert string => version
    #[error("unable to convert a string to `Version`")]
    FromString,
}

mod test {
    use super::Version;
    use anyhow::Result;

    #[test]
    fn version_greater_manual() {
        let v1 = Version::new(0, 7, 1);
        let v2 = Version::new(0, 7, 2);
        assert!(v2.greater_than(v1));

        let v1 = Version::new(1, 0, 1);
        let v2 = Version::new(0, 7, 2);
        assert!(v1.greater_than(v2));

        let v1 = Version::new(0, 0, 2);
        let v2 = Version::new(0, 3, 0);
        assert!(v2.greater_than(v1));
    }

    #[test]
    fn version_lesser_manual() {
        let v1 = Version::new(0, 7, 1);
        let v2 = Version::new(0, 7, 2);
        assert!(v1.less_than(v2));

        let v1 = Version::new(1, 0, 1);
        let v2 = Version::new(0, 7, 2);
        assert!(v2.less_than(v1));

        let v1 = Version::new(0, 0, 2);
        let v2 = Version::new(0, 3, 0);
        assert!(v1.less_than(v2));
    }

    #[test]
    fn version_greater_macro() {
        let v1 = Version::new(0, 7, 1);
        let v2 = Version::new(0, 7, 2);
        assert!(v2 > v1);

        let v1 = Version::new(1, 0, 1);
        let v2 = Version::new(0, 7, 2);
        assert!(v1 > v2);

        let v1 = Version::new(1, 0, 2);
        let v2 = Version::new(0, 3, 0);
        assert!(v1 > v2);
    }

    #[test]
    fn version_lesser_macro() {
        let v1 = Version::new(0, 7, 1);
        let v2 = Version::new(0, 7, 2);
        assert!(v1 < v2);

        let v1 = Version::new(0, 0, 1);
        let v2 = Version::new(0, 7, 2);
        assert!(v1 < v2);

        let v1 = Version::new(1, 0, 2);
        let v2 = Version::new(0, 3, 0);
        assert!(v2 < v1);
    }

    #[test]
    fn version_equal() {
        let v1 = Version::new(0, 7, 1);
        let v2 = Version::new(0, 7, 1);
        assert!(v1 == v2);

        let v1 = Version::new(0, 0, 1);
        let v2 = Version::new(0, 0, 1);
        assert!(v1 == v2);

        let v1 = Version::new(0, 0, 0);
        let v2 = Version::new(0, 0, 0);
        assert!(v1 == v2);
    }
}
