//! Contains functions that act on [`Txn`] to modify the queries stored in the
//! `query` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS query (
//!     text TEXT NOT NULL PRIMARY KEY
//! );
//! ```

use super::{
    sqlbuilder::{Sort, SqlBuilder},
    types::{
        file::{File, FileId, Files, MimeType},
        query::{Queries, Query},
        ID,
    },
    Error, Txn,
};
use crate::{fail, query_fail};
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use colored::Colorize;
use mime::Mime;
use std::{
    convert::{TryFrom, TryInto},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    time::SystemTime,
};

use rusqlite::{self as rsq, params};

// ================================ Txn ===============================
// =========================== Query Actions ==========================

impl Txn<'_> {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Return all [`Query`] in the database
    pub(crate) fn queries(&self) -> Result<Queries> {
        let queries: Vec<Query> = self
            .query_vec(
                "SELECT text
                FROM query
                ORDER BY text",
                params![],
                |row| row.try_into().expect("failed to convert to `Query`"),
            )
            .context(query_fail!("`Queries`"))?;

        Ok(queries.into())
    }

    /// Retrieve a [`Query`] that matches the given text
    pub(crate) fn query<S: AsRef<str>>(&self, q: S) -> Result<Query> {
        let query: Query = self
            .select(
                "SELECT text
                FROM query
                WHERE text = ?1",
                params![q.as_ref()],
                |row| {
                    let r: Query = row.try_into().expect("failed to convert to `Query`");
                    Ok(r)
                },
            )
            .context(query_fail!("`Query`"))?;

        Ok(query)
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a [`Query`] into the query table. Returns a [`Query`]
    pub(crate) fn insert_query<S: AsRef<str>>(&self, q: S) -> Result<Query> {
        let q = q.as_ref();
        self.insert(
            "INSERT INTO query (text)
            VALUES (?1)",
            params![q],
        )
        .context(fail!("insert `Query`"))?;

        Ok(Query::new(q))
    }

    /// Delete a [`Query`] from the query table
    pub(crate) fn delete_query<S: AsRef<str>>(&self, q: S) -> Result<()> {
        self.execute(
            "DELETE FROM query
            WHERE text = ?",
            params![q.as_ref()],
        )
        .context(fail!("delete `Query`"))?;

        Ok(())
    }
}
