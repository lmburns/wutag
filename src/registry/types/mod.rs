//! Types used for the [`Registry`](super::Registry) database.
//! All objects within this module are database objects

pub(crate) mod file;
pub(crate) mod filetag;
pub(crate) mod implication;
pub(crate) mod query;
pub(crate) mod tag;
pub(crate) mod tag_color;
pub(crate) mod value;

pub(crate) use file::{File, FileId, Files};
pub(crate) use filetag::{FileTag, FileTags};
pub(crate) use implication::{Implication, Implications};
pub(crate) use tag::{Tag, TagId, Tags};
pub(crate) use value::{Value, ValueId, Values};

use chrono::{DateTime, Local};
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ╭──────────────────────────────────────────────────────────╮
// │                         Property                         │
// ╰──────────────────────────────────────────────────────────╯

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

// ╭──────────────────────────────────────────────────────────╮
// │                            ID                            │
// ╰──────────────────────────────────────────────────────────╯

/// A row `ID`
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub(crate) struct ID(i64);

impl ID {
    /// Returns an invalid [`ID`]
    pub(crate) const fn null() -> Self {
        Self(0)
    }

    /// Returns the largest [`ID`]
    pub(crate) const fn max() -> Self {
        Self(i64::MAX)
    }

    /// Returns the inner [`ID`]
    pub(crate) const fn id(self) -> i64 {
        self.0
    }

    /// Create a new [`ID`] from a number
    pub(crate) const fn new(id: i64) -> Self {
        Self(id)
    }
}

impl Default for ID {
    fn default() -> Self {
        Self::null()
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

impl From<ID> for i64 {
    #[inline]
    fn from(id: ID) -> Self {
        id.id()
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

// ╭──────────────────────────────────────────────────────────╮
// │                         ModType                          │
// ╰──────────────────────────────────────────────────────────╯

/// An operation to the database
#[derive(Debug, PartialEq, Clone, Eq)]
pub(crate) struct Operation {
    ty:      ModType,
    table:   Table,
    literal: String,
    uuid:    Uuid,
}

/// The type of modification carried out on the database
#[derive(Debug, PartialEq, Clone, Eq)]
pub(crate) enum ModType {
    Add,
    Delete,
    Update,
}

impl ToSql for ModType {
    fn to_sql(&self) -> rsq::Result<ToSqlOutput<'_>> {
        match self {
            Self::Add => "add",
            Self::Delete => "delete",
            Self::Update => "update",
        }
        .to_sql()
    }
}

impl FromSql for ModType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "add" => Ok(Self::Add),
            "delete" => Ok(Self::Delete),
            "update" => Ok(Self::Update),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

// ╭──────────────────────────────────────────────────────────╮
// │                          Table                           │
// ╰──────────────────────────────────────────────────────────╯

/// Tables that are in the database
#[derive(Debug, Clone, PartialEq, Eq)]
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

// ╭──────────────────────────────────────────────────────────╮
// │                          Extra                           │
// ╰──────────────────────────────────────────────────────────╯

/// Convert a single type (e.g., [`Tag`]) to a type that is a wrapper for a
/// vector of that same type (e.g., [`Tags`])
///
/// [`Tag`]: ./tag/struct.Tag.html
/// [`Tags`]: ./tag/struct.Tags.html
macro_rules! from_vec {
    ($from:tt, $to:tt) => {
        impl From<Vec<$from>> for $to {
            fn from(t: Vec<$from>) -> Self {
                Self::new(t)
            }
        }
    };
}

/// Base functions to use for structs that consist only of an inner vector.
///
/// Used specifically for objects that are serialized form a [`rusqlite`]
/// database.
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

        /// Clear the values in the vector
        pub(crate) fn clear(&mut self) {
            self.inner.clear();
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

        /// Test whether the inner vector is empty
        pub(crate) fn is_empty(&self) -> bool {
            self.inner.is_empty()
        }

        /// Get an item from the inner vector
        pub(crate) fn get(&self, idx: usize) -> Option<&$t> {
            self.inner.get(idx)
        }

        /// Get unique items from the inner array
        pub(crate) fn unique(&mut self) {
            self.inner.sort_unstable();
            self.inner.dedup();
        }

        /// Get unique items from the inner array, returning `Self`
        pub(crate) fn unique_b(mut self) -> Self {
            self.inner.sort_unstable();
            self.inner.dedup();
            self
        }

        /// Return an iterator of the inner vector
        pub(crate) fn iter(&self) -> std::slice::Iter<'_, $t> {
            self.inner.iter()
        }

        /// Shorthand to the [`itertools`] function [`find_position`]
        ///
        /// [`find_position`]: itertools::Itertools::find_position
        pub(crate) fn find_position<F>(&self, mut f: F) -> Option<(usize, &$t)>
        where
            F: FnMut(&$t) -> bool,
        {
            use itertools::Itertools;
            self.iter().find_position(|item| f(*item))
        }

        /// Shorthand to the Rust builtin [`filter`]
        ///
        /// [`any`]: std::iter::Iterator::filter
        pub(crate) fn filter<F>(&self, mut f: F) -> Vec<$t>
        where
            F: FnMut(&$t) -> bool,
        {
            self.iter().filter(|i| f(i)).cloned().collect::<Vec<$t>>()
        }

        /// Shorthand to the Rust builtin [`any`]
        ///
        /// [`any`]: std::iter::Iterator::any
        pub(crate) fn any<F>(&self, f: F) -> bool
        where
            F: Fn(&$t) -> bool,
        {
            self.iter().any(f)
        }

        /// Shorthand to the Rust builtin [`all`]
        ///
        /// [`any`]: std::iter::Iterator::all
        pub(crate) fn all<F>(&self, f: F) -> bool
        where
            F: Fn(&$t) -> bool,
        {
            self.iter().all(f)
        }

        /// Same function as [`map`] on any iterator. Returns a vector
        ///
        /// [`map`]: std::iter::Iterator::map
        pub(crate) fn map_vec<F, B>(&self, mut f: F) -> Vec<B>
        where
            F: FnMut(&$t) -> B,
        {
            self.iter().map(|i| f(&i)).collect::<Vec<B>>()
        }

        /// Same function as [`map`] on any iterator.
        /// Returns a **unique** vector
        ///
        /// [`map`]: std::iter::Iterator::map
        pub(crate) fn map_vec_uniq<F, B>(&self, mut f: F) -> Vec<B>
        where
            B: Clone + Eq + std::hash::Hash,
            F: FnMut(&$t) -> B,
        {
            use itertools::Itertools;
            self.iter().map(f).unique().collect::<Vec<B>>()
        }
    };
    ($impl:tt, $t:tt) => {
        impl $impl {
            impl_vec!($t);
        }
    };
}

// macro_rules! impl_intoiter {
//     ($t:tt, $type:tt) => {
//         impl IntoIterator for $t {
//             type IntoIter = std::vec::IntoIter<$type>;
//             type Item = $type;
//
//             fn into_iter(self) -> Self::IntoIter {
//                 self.inner().into_iter()
//             }
//         }
//     };
// }

// macro_rules! impl_deref {
//     ($t:tt, $type:tt) => {
//         impl std::ops::Deref for $t {
//             type Target = Vec<$type>;
//
//             fn deref(&self) -> &Self::Target {
//                 &self.inner
//             }
//         }
//     };
// }

/// Create a function that will validate the given type's name before inserting
/// it into the database.
///
/// This helps to prevent *some* parsing errors with `nom`
macro_rules! validate_name {
    ($type:tt, $type_plural:tt) => {
        /// Verify that the given name is valid
        pub(crate) fn validate_name<S: AsRef<str>>(name: S) -> anyhow::Result<()> {
            use anyhow::anyhow;
            use regex::Regex;
            use $crate::registry::querier::{
                COMPARISON_OPS, CONDITIONAL_RES, FUNC_NAMES, OTHER_RES,
            };

            /// Color the items in an array
            macro_rules! color_arr {
                ($arr:expr) => {
                    $arr.iter()
                        .map(|s| s.red().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
            }

            let name = name.as_ref().to_ascii_lowercase();
            if COMPARISON_OPS.iter().copied().any(|c| c == name) {
                return Err(anyhow!(
                    "invalid {} was given: {}\n\nValid {} must not contain any operators:\n\t- {}",
                    $type,
                    name,
                    $type_plural,
                    color_arr!(COMPARISON_OPS)
                ));
            }

            if CONDITIONAL_RES.iter().copied().any(|r| r == name) {
                return Err(anyhow!(
                    "invalid {} was given: {}\n\nValid {} must not contain any conditional \
             keywords:\n\t- {}",
                    $type,
                    name,
                    $type_plural,
                    color_arr!(CONDITIONAL_RES)
                ));
            }

            let reg = $crate::regex!(&format!("({})\\(.*\\)", FUNC_NAMES.join("|")));
            if reg.is_match(&name) {
                return Err(anyhow!(
                    "invalid {} was given: {}\n\nValid {} must not contain any of the following \
             function names, proceeded by an opening and closing parenethesis:\n\t- {}",
                    $type,
                    name,
                    $type_plural,
                    color_arr!(FUNC_NAMES)
                ));
            }

            if OTHER_RES
                .iter()
                .any(|reg| $crate::regex!(reg).is_match(&name))
            {
                return Err(anyhow!(
                    "invalid {} was given: {}\n\nValid {} must not contain any of:\n\t- {}",
                    $type,
                    name,
                    $type_plural,
                    color_arr!(
                        ([
                            "$F",
                            "@F",
                            "@F[N]",
                            "@F[N..M]",
                            "@F[N,M]",
                            "@F[N..=M]",
                            "@F[..]",
                            "@F[..=M]",
                            "%r//",
                            "%g//",
                            "//r",
                            "//g"
                        ])
                    )
                ));
            }

            Ok(())
        }
    };
}

// A `pub` qualifier prevents the macro from only being accessible from the
// crate root
pub(crate) use from_vec;
pub(crate) use impl_vec;
pub(crate) use validate_name;
