//! Versioning for the crate and database

#![allow(unused)]

use anyhow::{Context, Result};

// ===================== Version ======================

/// Crate version -- used for making sure the user's database schema is up to
/// date and will be compatible with the current version of `wutag`
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    major: usize,
    minor: usize,
    patch: usize,
}

impl Version {
    /// Create a `Version` from specified values. Used for creating a `Version`
    /// from a database query
    pub(crate) fn new(major: usize, minor: usize, patch: usize) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Create the `Version` struct from the crate's version
    #[allow(clippy::self_named_constructors)]
    pub(crate) fn version() -> Result<Self> {
        let version = env!("CARGO_PKG_VERSION");
        let split = version
            .split('.')
            .map(|v| v.parse::<usize>().expect("failed to parse version"))
            .collect::<Vec<_>>();

        Ok(Self {
            major: *(split.get(0).context("failed to get idx: 0")?),
            minor: *(split.get(1).context("failed to get minor")?),
            patch: *(split.get(2).context("failed to get idx: 2")?),
        })
    }

    /// Return the `major` version
    pub(crate) fn major(&self) -> usize {
        self.major
    }

    /// Return the `minor` version
    pub(crate) fn minor(&self) -> usize {
        self.minor
    }

    /// Return the `patch` version
    pub(crate) fn patch(&self) -> usize {
        self.patch
    }

    /// Determine if the `Version` is less than another
    pub(crate) fn less_than(&self, other: Self) -> bool {
        // self < &other
        (self.major < other.major)
            || (self.major == other.major && self.minor < other.minor)
            || (self.major == other.major && self.minor == other.minor && self.patch < other.patch)
    }

    /// Determine if the `Version` is greater than another
    pub(crate) fn greater_than(&self, other: Self) -> bool {
        // self > &other
        (self.major > other.major)
            || (self.major == other.major && self.minor > other.minor)
            || (self.major == other.major && self.minor == other.minor && self.patch > other.patch)
    }
}
