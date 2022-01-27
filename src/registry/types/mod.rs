//! Types used for the [`Registry`](super::Registry)

pub(crate) mod file;
pub(crate) mod filetag;
pub(crate) mod implication;
pub(crate) mod query;
pub(crate) mod tag;
pub(crate) mod tag_color;
pub(crate) mod value;

use chrono::{DateTime, Local};
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================= Property =============================
// ====================================================================

/// All types that can be translated from the database
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Property {
    /// A number
    Number(i64),
    /// A string
    String(String),
    /// A boolean
    Bool(bool),
    /// A `DateTime`
    Datetime(DateTime<Local>),
}

#[allow(clippy::wildcard_enum_match_arm)]
impl Property {
    /// Convert the `Number` type to an `i64`
    pub(crate) const fn as_number(&self) -> Option<i64> {
        match &self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Convert the `String` type to a `String`
    pub(crate) fn as_string(&self) -> Option<String> {
        match &self {
            Self::String(s) => Some(s.to_string()),
            _ => None,
        }
    }

    /// Convert the `Bool` type to a `bool`
    pub(crate) const fn as_bool(&self) -> Option<bool> {
        match &self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Convert the `Datetime` type to a `DateTime`
    pub(crate) const fn as_datetime(&self) -> Option<DateTime<Local>> {
        match &self {
            Self::Datetime(d) => Some(*d),
            _ => None,
        }
    }
}

// ================================ ID ================================
// ====================================================================

/// A row `ID`
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ID(i64);

impl ID {
    /// Returns an invalid `ID`
    pub(crate) const fn null() -> Self {
        Self(0)
    }

    /// Returns the largest `ID`
    pub(crate) const fn max() -> Self {
        Self(i64::MAX)
    }

    /// Returns the inner `ID`
    pub(crate) const fn id(self) -> i64 {
        self.0
    }

    /// Create a new `ID` from a number
    pub(crate) const fn new(id: i64) -> Self {
        Self(id)
    }
}

impl ToSql for ID {
    fn to_sql(&self) -> rsq::Result<ToSqlOutput> {
        Ok(ToSqlOutput::from(self.0))
    }
}

impl FromSql for ID {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value.as_i64().map(Into::into)
    }
}

impl From<i64> for ID {
    #[inline]
    fn from(id: i64) -> Self {
        Self(id)
    }
}

impl From<ID> for ToSqlOutput<'_> {
    #[inline]
    fn from(t: ID) -> Self {
        Self::Owned(t.id().into())
    }
}

impl fmt::Display for ID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// impl<'a> From<&'a str> for Property {
//     fn from(s: &str) -> Self {
//         Property::String(s.to_string())
//     }
// }
//
// impl From<String> for Property {
//     fn from(s: String) -> Self {
//         Property::String(s.clone())
//     }
// }
//
// impl From<&String> for Property {
//     fn from(s: &String) -> Self {
//         Property::String(s.clone())
//     }
// }
//
// impl From<i64> for Property {
//     fn from(s: i64) -> Self {
//         Property::Number(s.clone())
//     }
// }

// ============================ Operation =============================
// ====================================================================

/// A change to the database
#[allow(variant_size_differences)]
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Operation1 {
    Add {
        id: ID,
    },
    Delete {
        id: ID,
    },
    Update {
        id:        ID,
        property:  String,
        value:     Option<String>,
        timestamp: DateTime<Local>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Operation {
    Add,
    Delete,
    Update,
}

impl ToSql for Operation {
    fn to_sql(&self) -> rsq::Result<ToSqlOutput<'_>> {
        match self {
            Self::Add => "add",
            Self::Delete => "delete",
            Self::Update => "update",
        }
        .to_sql()
    }
}

impl FromSql for Operation {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "add" => Ok(Self::Add),
            "delete" => Ok(Self::Delete),
            "update" => Ok(Self::Update),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

// ============================== Table ===============================
// ====================================================================

/// Tables that are in the database
#[derive(Debug, Clone)]
pub(crate) enum Table {
    Tag,
    File,
    Value,
    FileTag,
    Implication,
    Query,
    Tracker,
    Checkpoint,
    Version,
}

impl ToSql for Table {
    fn to_sql(&self) -> rsq::Result<ToSqlOutput<'_>> {
        match self {
            Self::Tag => "tag",
            Self::File => "file",
            Self::Value => "value",
            Self::FileTag => "file_tag",
            Self::Implication => "impl",
            Self::Query => "query",
            Self::Tracker => "tracker",
            Self::Checkpoint => "checkpoint",
            Self::Version => "version",
        }
        .to_sql()
    }
}

impl FromSql for Table {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "tag" => Ok(Self::Tag),
            "file" => Ok(Self::File),
            "value" => Ok(Self::Value),
            "file_tag" => Ok(Self::FileTag),
            "impl" => Ok(Self::Implication),
            "query" => Ok(Self::Query),
            "tracker" => Ok(Self::Tracker),
            "checkpoint" => Ok(Self::Checkpoint),
            "version" => Ok(Self::Version),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

// ============================== Extra ===============================
// ====================================================================

/// Convert a single type (e.g., `Tag`) to its' type that is a vector of that
/// same type (e.g., `Tags`)
///
/// `Tag`: self::tag::Tag
/// `Tags`: self::tag::Tags
macro_rules! from_vec {
    ($from:tt, $to:tt) => {
        impl From<Vec<$from>> for $to {
            fn from(t: Vec<$from>) -> Self {
                Self::new(t)
            }
        }
    };
}

/// Base functions to use for structs that consist only of an inner vector
macro_rules! impl_vec {
    ($t:tt) => {
        /// Create a new set from a vector
        pub(crate) fn new(v: Vec<$t>) -> Self {
            Self { inner: v }
        }

        /// Create a new blank set
        pub(crate) const fn empty() -> Self {
            Self { inner: vec![] }
        }

        /// Extend the inner vector
        pub(crate) fn extend(&mut self, v: &[$t]) {
            self.inner.extend_from_slice(v);
        }

        /// Add an item to the set
        pub(crate) fn push(&mut self, t: $t) {
            self.inner.push(t);
        }

        /// Return the inner vector
        pub(crate) fn inner(&self) -> &[$t] {
            &self.inner
        }

        /// Combine with another object
        pub(crate) fn combine(&mut self, other: &Self) {
            self.extend(other.inner());
        }

        /// Return the length of the inner vector
        pub(crate) fn len(&self) -> usize {
            self.inner.len()
        }
    };
    ($impl:tt, $t:tt) => {
        impl $impl {
            impl_vec!($t);
        }
    };
}

// A `pub` qualifier prevents the macro from only being accessible from the
// crate root
pub(crate) use from_vec;
pub(crate) use impl_vec;
