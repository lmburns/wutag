#![deny(broken_intra_doc_links)]
//! Intermediate state of changes that have yet to be committed to the
//! [`Registry`](super::Registry)

use super::{
    common::version::Version,
    sqlbuilder::SqlBuilder,
    types::{file::File, ModType, Table, ID},
    Error, Registry,
};
use crate::{fail, failt};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use rusqlite::{
    self as rsq, params,
    types::{FromSql, ToSql, Value},
    Connection, DropBehavior,
    Error::{SqliteFailure, StatementChangedRows},
    Params, ParamsFromIter, Row, Savepoint, Transaction,
};
use std::{cell::RefCell, fmt};

use once_cell::unsync::{Lazy, OnceCell};

// ╒══════════════════════════════════════════════════════════╕
//                         Transaction
// ╘══════════════════════════════════════════════════════════╛

/// This is an intermediate stage between a command and a change to the
/// database. See [`Transaction`](https://www.sqlite.org/lang_transaction.html)
#[derive(Debug)]
pub(crate) struct Txn<'t> {
    /// Should symlinks be followed?
    follow_symlinks: bool,
    /// A [`Transaction`] on a database, which allows for modifications
    pub(crate) txn:  Transaction<'t>,
}

impl<'t> Txn<'t> {
    /// Create a new [`Txn`]
    pub(crate) fn new(conn: &'t Connection, follow_symlinks: bool) -> Result<Self> {
        log::trace!("creating new Transaction");

        let mut txn = conn.unchecked_transaction()?;
        txn.set_drop_behavior(DropBehavior::Commit);

        Ok(Self {
            follow_symlinks,
            txn,
        })
    }

    /// Return the [`Transaction`] by taking ownership of `self`
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn txn(self) -> Transaction<'t> {
        self.txn
    }

    /// Return the [`Transaction`] as a reference
    pub(crate) const fn txn_ref(&'t self) -> &'t Transaction<'t> {
        &self.txn
    }

    /// Return whether the user prefers to follow symlinks
    pub(crate) const fn follow_symlinks(&self) -> bool {
        self.follow_symlinks
    }

    /// Commit a [`Transaction`]
    pub(crate) fn commit(self) -> Result<()> {
        // TODO: Create checkpoint here
        self.txn.commit().context(fail!("committing transaction"))
    }

    // Checkout rollback/commit hooks
    // TODO: Implement an ability to rollback

    /// Rollback a [`Transaction`]
    pub(crate) fn rollback(self) -> Result<()> {
        self.txn.rollback().context(failt!("rollback transaction"))
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Executing                         │
    // ╰──────────────────────────────────────────────────────────╯

    /// Execute a command. Implements the same function as [`execute`].
    /// Returns the number of rows that are affected by the change
    ///
    /// [`execute`]: /rusqlite/struct.Connection.html#method.execute
    pub(crate) fn execute<P: Params>(&self, sql: &str, params: P) -> Result<usize> {
        log::trace!("{}({}): {}", "execute".green().bold(), "Txn".purple(), sql);

        self.txn
            .execute(sql, params)
            .context(fail!("executing command: {}", sql))
    }

    /// Execute a command with no [`params`]. Implements the same function as
    /// [`execute`]
    ///
    /// [`execute`]: /rusqlite/struct.Connection.html#method.execute
    /// [`params`]: rusqlite::params
    pub(crate) fn exec_no_params(&self, sql: &str) -> Result<()> {
        self.execute(sql, params![])?;

        Ok(())
    }

    /// Return true if the SQL statement returns a row, else false
    /// Identical to [`exists`]. Used for checking whether a table exists to
    /// tell the user it isn't initialized
    ///
    /// [`exists`]: /rusqlite/statement/struct.Statement.html#method.exists
    pub(crate) fn exists(&self, tbl: &str) -> Result<()> {
        let sql = format!("SELECT * FROM {}", tbl);

        // .context() Didn't show error here
        if self.txn.prepare(&sql).is_err() {
            return Err(anyhow!("Please make sure to run 'init' before using wutag"));
        }

        Ok(())
    }

    /// Insert a row into the database. Implements the same function as
    /// [`insert`]. Returns the [`last_insert_rowid`]
    ///
    /// [`insert`]: /rusqlite/statement/struct.Statement.html#method.insert
    /// [`last_insert_rowid`]: /rusqlite/struct.Connection.html#method.last_insert_rowid
    pub(crate) fn insert<P: Params>(&self, sql: &str, params: P) -> Result<i64> {
        log::trace!("{}({}): {}", "insert".green().bold(), "Txn".purple(), sql);

        // Check whether the tag table exists
        self.exists("tag")?;

        let mut stmt = self
            .txn
            .prepare(sql)
            .context(fail!("preparing sql: {}", sql))?;

        let res = stmt
            .insert(params)
            .context(fail!("inserting item: {}", sql));

        if let Err(err) = res {
            // Check if it is a unique constraint violation
            return match err.downcast::<rsq::Error>() {
                Ok(SqliteFailure(e, Some(ctx))) => return Err(anyhow!("{}: {}", e, ctx)),
                Ok(e) => return Err(anyhow::Error::from(e)),
                // Ok(StatementChangedRows(n)) => Ok(n as i64),
                _ => Err(anyhow!("failed to downcast error")),
            };
        }

        res
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Retrieving                        │
    // ╰──────────────────────────────────────────────────────────╯

    /// Select a single row. Implements the same function as [`query_row`]
    ///
    /// [`query_row`]: /rusqlite/statement/struct.Statement#method.query_row
    pub(crate) fn select<T, F, P>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> Result<T, rsq::Error>,
    {
        log::trace!("{}({}): {}", "select".green().bold(), "Txn".purple(), sql);

        self.exists("tag")?;

        let mut stmt = self
            .txn
            .prepare_cached(sql)
            .context(failt!("prepare sql: {}", sql))?;

        stmt.query_row(params, f)
            .context(failt!("select row: {}", sql))
    }

    /// Select a single row, no [`params`], and no closure
    pub(crate) fn select1<T: FromSql>(&self, sql: &str) -> Result<T> {
        self.select(sql, params![], |row| row.get(0))
    }

    /// Select all matching rows. Implements a function on each of these
    /// matches, returning a vector of type `T`. Implements [`query`] but is
    /// closer in equivalency to [`query_map`]
    ///
    /// [`query`]: /rusqlite/statement/struct.Statement.html#method.query
    /// [`query_map`]: /rusqlite/statement/struct.Statement.html#method.query_map
    pub(crate) fn query_vec<T, F, P, S>(&self, sql: S, params: P, f: F) -> Result<Vec<T>>
    where
        S: AsRef<str>,
        P: Params,
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        // Used for ergonomics
        let sql = sql.as_ref();

        log::trace!(
            "{}({}): {}",
            "query_vec".green().bold(),
            "Txn".purple(),
            sql
        );

        self.exists("tag")?;

        let mut stmt = self
            .txn
            .prepare_cached(sql)
            .context(failt!("prepare sql: {}", sql))?;
        let mut rows = stmt
            .query(params)
            .context(fail!("querying row(s): {}", sql))?;

        let mut v = Vec::<T>::new();
        while let Some(row) = rows.next().context(fail!("getting next item"))? {
            v.push(f(row));
        }

        Ok(v)
    }

    /// Select all items matching a query. This function requires that `params`
    /// are an iterator implementing [`ToSql`]. Implements the same function as
    /// [`query_map`] on this iterator.
    pub(crate) fn query_iter<T, F, P, S>(&self, sql: S, params: P, f: F) -> Result<Vec<T>>
    where
        S: AsRef<str>,
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        let sql = sql.as_ref();

        log::trace!(
            "{}({}): {}",
            "query_iter".green().bold(),
            "Txn".purple(),
            sql
        );

        self.exists("tag")?;

        let mut stmt = self
            .txn
            .prepare_cached(sql)
            .context(failt!("prepare sql: {}", sql))?;
        let mut rows = stmt
            .query(rsq::params_from_iter(params))
            .context(failt!("query_iter: {}", sql))?;

        let mut v = Vec::<T>::new();
        while let Some(row) = rows.next().context(fail!("getting next item"))? {
            v.push(f(row));
        }

        Ok(v)
    }

    /// Shorter function that allows for querying the database from an
    /// [`SqlBuilder`]. Calls [`query_iter`](#method.query_iter) internally
    pub(crate) fn query_builder<T, F>(&self, builder: &SqlBuilder, f: F) -> Result<Vec<T>>
    where
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        self.query_iter(builder.utf()?, builder.params_as_slice(), f)
    }

    /// Shorter function that allows for querying the database from an
    /// [`SqlBuilder`]. Calls [`query_vec`](#method.query_vec) internally on a
    /// named parameters vector, i.e., `&[(&str, &dyn ToSql)]`
    pub(crate) fn query_named_builder<T, F>(&self, builder: &SqlBuilder, f: F) -> Result<Vec<T>>
    where
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        self.query_vec(builder.utf()?, builder.named_params_as_slice(), f)
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                         Version                          │
    // ╰──────────────────────────────────────────────────────────╯

    // TODO: Remove these if not used

    /// Insert the latest version into the database
    pub(crate) fn insert_version(&self) -> Result<()> {
        let v = Version::build().context(fail!("getting current version"))?;
        log::trace!("inserting Version({})", v);

        // TODO: Should context wrap this?
        self.insert(
            "INSERT INTO version (major, minor, patch)
                VALUES (?1, ?2, ?3)",
            params![v.major(), v.minor(), v.patch()],
        )
        .context(fail!("inserting Version into version table"))?;

        Ok(())
    }

    /// Get the current version of the database
    pub(crate) fn get_current_version(&self) -> Result<Version> {
        let res = self
            .select("SELECT * from version", params![], |row| {
                Ok(Version::new(row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .context(fail!("querying row"))?;

        Ok(res)
    }

    /// Update the current version of the database
    pub(crate) fn update_current_version(&self) -> Result<()> {
        let v = Version::build().context(fail!("getting current version"))?;
        log::trace!("updating Version({})", v);

        self.execute(
            "UPDATE version SET major = ?1, minor = ?2, patch = ?3",
            params![v.major(), v.minor(), v.patch()],
        )
        .context(fail!("updating current Version"))?;

        Ok(())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                     Tracking Actions                     │
    // ╰──────────────────────────────────────────────────────────╯

    /// Insert a change into the `tracker` table, which keeps tracks of actions
    /// modifying anything in the entire database
    pub(crate) fn record_modification<S: AsRef<str>>(
        &self,
        table: &Table,
        op: &ModType,
        id: ID,
        previous: S,
    ) -> Result<usize> {
        log::trace!("recording modification");
        self.execute(
            "INSERT INTO tracker (table, operation, operation_id, previous)
            VALUES (?1, ?2, ?3, ?4)",
            params![table, op, id, previous.as_ref()],
        )
        .context(failt!("record a change"))
    }

    /// Create a checkpoint in the `checkpoint` table
    pub(crate) fn create_checkpoint<S: AsRef<str>>(&self, desc: S) -> Result<usize> {
        let desc = desc.as_ref();
        let debug = format!("inserting Checkpoint({})", desc);
        self.execute(
            "INSERT INTO checkpoint (tracker_id, description)
            VALUES (
                (
                    SELECT
                        IFNULL(MAX(tracker_id), 0)
                    FROM tracker
                ),
                ?1
            )",
            params![desc],
        )
        .context(fail!("{}", debug))
    }
}
