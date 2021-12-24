//! Contains functions that act on `Txn` to modify the queries stored in the
//! `query` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS query (
//!     sha BINARY(32) PRIMARY KEY,
//!     text TEXT NOT NULL
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
            .context("failed to query for `Queries`")?;

        Ok(queries.into())
    }

    // ============================= Modifying ============================
    // ====================================================================
}
