//! Structure that holds an `SQL` query and its parameters

use anyhow::{anyhow, Context, Result};
use bytes::{Bytes, BytesMut};
use itertools::Itertools;
use rusqlite::{
    self as rsq, params,
    types::{ToSql, ToSqlOutput},
    Params,
};
use std::{
    fmt::{self, Write},
    ops::Deref,
};

// =================== SqlBuilder =====================

/// Builder for an `SQL` query
pub(crate) struct SqlBuilder {
    /// The `SQL` query as bytes
    query:  BytesMut,
    /// Parameters used for the `SQL` query
    params: Vec<Box<dyn ToSql>>,
    /// The index of the parameters
    pidx:   usize,
    /// Does the query need a comma? (i.e., there's more than one param)
    comma:  bool,
}

impl SqlBuilder {
    /// Create a new `SqlBuilder
    pub(crate) fn new() -> Self {
        Self {
            query:  BytesMut::new(),
            params: vec![],
            pidx:   1,
            comma:  false,
        }
    }

    // /// Return the `query` as a `Bytes` buf
    // pub(crate) fn as_buf(&self) -> &Bytes {
    //     &*self.query.freeze()
    // }

    /// Return the `query` as bytes
    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        self.query.to_vec()
    }

    /// Return the `query` as a `String`
    pub(crate) fn utf(&self) -> Result<String> {
        String::from_utf8(self.query.to_vec()).context("failed to convert query to String")
    }

    /// Return the `params` as a vector of [`ToSqlOutput`]
    pub(crate) fn params_as_output(&self) -> Result<Vec<ToSqlOutput<'_>>> {
        self.params
            .iter()
            .map(|s| ToSql::to_sql(s).map_err(|e| anyhow!(e)))
            .into_iter()
            .collect()
    }

    /// Return the `params` as a slice where each element implements [`ToSql`]
    pub(crate) fn params_as_slice(&self) -> Vec<&dyn ToSql> {
        self.params.iter().map(Deref::deref).collect::<Vec<_>>()
    }

    /// Append a string to the query with a starting newline
    pub(crate) fn appendln<S: AsRef<str>>(&mut self, s: S) {
        // let chars = s.chars().collect::<Vec<_>>();
        // if chars[0] == ' ' || chars[0] == '\n' {}

        self.query.write_str("\n");
        self.query.write_str(s.as_ref());
        self.comma = false;
    }

    /// Append a string to the query
    pub(crate) fn append<S: AsRef<str>>(&mut self, s: S) {
        self.query.write_str(s.as_ref());
        self.comma = false;
    }

    /// Append a parameter to the vector of `params`
    pub(crate) fn append_param<S: ToSql + 'static>(&mut self, param: S) {
        if self.comma {
            self.query.write_str(",");
        }

        self.query.write_str(&format!("?{}", self.pidx));
        self.pidx += 1;

        self.params.push(Box::new(param));
        self.comma = true;
    }

    /// Append `COLLATE NOCASE` to ignore case when searching
    pub(crate) fn nocase_collation(&mut self, ignore: bool) {
        if ignore {
            self.query.write_str(" COLLATE NOCASE ");
        }
    }
}

impl fmt::Debug for SqlBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqlBuilder")
            .field("query", &self.query.to_vec())
            .field(
                "params",
                &self.params.iter().fold(String::new(), |mut acc, f| {
                    acc.push_str(&format!(" {:?}", f.to_sql()));
                    acc
                }),
            )
            .field("pidx", &self.pidx)
            .field("comma", &self.comma)
            .finish()
    }
}

// ====================== Sort ========================

/// The method in which the files should be sorted in the database
#[derive(Debug, Copy, Clone)]
pub(crate) enum Sort {
    /// Sort by the `File` id
    Id,
    /// Sort by the `File` name
    Name,
    /// Sort by the `File` `mtime`
    ModificationTime,
    /// Sort by the `File` `ctime`
    CreationTime,
    /// Sort by the `File` `size`
    FileSize,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Sort::Id => f.write_str("ORDER BY id"),
            Sort::Name => f.write_str("ORDER BY directory || '/' || name"),
            Sort::ModificationTime => f.write_str("ORDER BY mtime, directory || '/' || name"),
            Sort::CreationTime => f.write_str("ORDER BY ctime, directory || '/' || name"),
            Sort::FileSize => f.write_str("ORDER BY size, directory || '/' || name"),
        }
    }
}
