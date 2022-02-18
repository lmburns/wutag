//! A user's query into the database to return an item

use super::{
    super::{querier::ast::query::ParsedQuery, sqlbuilder::SqlBuilder},
    from_vec, impl_vec,
};
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};

// ============================== Query ===============================
// ====================================================================

/// Represents a query in the database
#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct Query {
    inner: String,
}

impl Query {
    /// Creates a new [`Query`]
    pub(crate) fn new<S: AsRef<str>>(q: S) -> Self {
        Self {
            inner: q.as_ref().to_owned(),
        }
    }
}

impl From<ParsedQuery> for Query {
    fn from(q: ParsedQuery) -> Self {
        Self::new(q.raw())
    }
}

impl From<String> for Query {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&String> for Query {
    fn from(s: &String) -> Self {
        Self::new(s.clone())
    }
}

impl From<&str> for Query {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<SqlBuilder<'_>> for Query {
    fn from(s: SqlBuilder) -> Self {
        Self {
            inner: s.utf().expect("invalid UTF-string in query"),
        }
    }
}

impl TryFrom<&Row<'_>> for Query {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: row.get("text")?,
        })
    }
}

// ============================= Queries ==============================
// ====================================================================

/// A vector of [`Query`]
#[derive(Debug, Clone)]
pub(crate) struct Queries {
    /// The inner vector of [`Query`]
    inner: Vec<Query>,
}

from_vec!(Query, Queries);

impl Queries {
    impl_vec!(Query);
}
