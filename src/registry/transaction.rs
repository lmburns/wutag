//! Intermediate state of changes that have yet to be committed to the
//! [`Registry`](super::Registry)

use super::{
    common::version::Version,
    sqlbuilder::SqlBuilder,
    types::{file::File, ModType, Table, ID},
    Registry,
};
use crate::fail;
use anyhow::{Context, Result};
use colored::Colorize;
use rusqlite::{
    self as rsq, params,
    types::{FromSql, ToSql, Value},
    Connection, Params, ParamsFromIter, Row, Transaction,
};

// =========================== Transaction ============================
// ====================================================================

/// This is an intermediate stage betweeen a command and a change to the
/// database. See [`Transaction`](https://www.sqlite.org/lang_transaction.html)
#[derive(Debug)]
pub(crate) struct Txn<'t> {
    /// The tag `Registry`
    registry: &'t Registry,
    /// A `Transaction` on a database, which allows for modifications
    txn:      Transaction<'t>,
}

impl<'t> Txn<'t> {
    // -- TODO: Possibly use this function inside every function instead of
    // initialization and then another function
    // -- TODO: Checked vs unchecked

    /// Create a new `Txn`
    pub(crate) fn new(registry: &'t Registry) -> Result<Self> {
        let txn = registry
            .conn()
            .unchecked_transaction()
            .context(fail!("get transaction"))?;

        Ok(Self { registry, txn })
    }

    /// Return the `Registry`
    pub(crate) const fn registry(&self) -> &'t Registry {
        self.registry
    }

    /// Return the `Transaction`
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn txn(self) -> Transaction<'t> {
        self.txn
    }

    /// Commit a [`Transaction`](rusqlite::Transaction)
    pub(crate) fn commit(self) -> Result<()> {
        self.txn.commit().context(fail!("commit transaction"))
    }

    /// Rollback a [`Transaction`](rusqlite::Transaction)
    pub(crate) fn rollback(self) -> Result<()> {
        self.txn.rollback().context(fail!("rollback transaction"))
    }

    // ============================ Executing =============================
    // ====================================================================

    /// Execute a command. Implements the same function as [`execute`].
    /// Returns the number of rows that are affected by the change
    ///
    /// [`execute`]: /rusqlite/struct.Connection.html#method.execute
    pub(crate) fn execute<P: Params>(&self, sql: &str, params: P) -> Result<usize> {
        log::debug!("{}({}): {}", "execute".green().bold(), "Txn".purple(), sql);

        self.txn
            .execute(sql, params)
            .context(fail!("execute command: {}", sql))
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

    /// Insert a row into the database. Implements the same function as
    /// [`insert`]. Returns the [`last_insert_rowid`]
    ///
    /// [`insert`]: /rusqlite/statement/struct.Statement.html#method.insert
    /// [`last_insert_rowid`]: /rusqlite/struct.Connection.html#method.last_insert_rowid
    pub(crate) fn insert<P: Params>(&self, sql: &str, params: P) -> Result<i64> {
        log::debug!("{}({}): {}", "insert".green().bold(), "Txn".purple(), sql);

        let mut stmt = self
            .txn
            .prepare(sql)
            .context(fail!("prepare sql: {}", sql))?;

        stmt.insert(params).context(fail!("insert item: {}", sql))
    }

    // ============================ Retrieving ============================
    // ====================================================================

    /// Select a single row. Implements the same function as [`query_row`]
    ///
    /// [`query_row`]: rusqlite::statement::Statement::query_row
    pub(crate) fn select<T, F, P>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> Result<T, rsq::Error>,
    {
        log::debug!("{}({}): {}", "select".green().bold(), "Txn".purple(), sql);
        let error = fail!("select row: {}", sql);

        let mut stmt = self
            .txn
            .prepare_cached(sql)
            .context(fail!("prepare sql: {}", sql))?;

        stmt.query_row(params, f).context(error)
    }

    /// Select a single row, no [`params`](rusqlite::params), and no closure
    pub(crate) fn select1<T: FromSql>(&self, sql: &str) -> Result<T> {
        self.select(sql, params![], |row| row.get(0))
    }

    /// Select all matching rows. Implements a function on each of these
    /// matches, returning a vector of type `T`. Implements [`query`] but is
    /// closer in equivalency to [`query_map`]
    ///
    /// [`query`]: rusqlite::statement::Statement::query
    /// [`query_map`]: rusqlite::statement::Statement::query_map
    pub(crate) fn query_vec<T, F, P, S>(&self, sql: S, params: P, f: F) -> Result<Vec<T>>
    where
        S: AsRef<str>,
        P: Params,
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        // Used for ergonomics
        let sql = sql.as_ref();

        log::debug!(
            "{}({}): {}",
            "query_vec".green().bold(),
            "Txn".purple(),
            sql
        );

        let mut stmt = self
            .txn
            .prepare_cached(sql)
            .context(fail!("prepare sql: {}", sql))?;
        let mut rows = stmt.query(params).context(fail!("query row(s): {}", sql))?;

        let mut v = Vec::<T>::new();
        while let Some(row) = rows.next().context(fail!("get next item"))? {
            v.push(f(row));
        }

        Ok(v)
    }

    /// Select all items matching a query. This function requires that `params`
    /// are an iterator implementing [`ToSql`]. Implements the same function as
    /// `query_map` on this iterator.
    ///
    /// `query_map`: rusqlite::statement::Statement::query_map
    pub(crate) fn query_iter<T, F, P, S>(&self, sql: S, params: P, f: F) -> Result<Vec<T>>
    where
        S: AsRef<str>,
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        // Used for ergonomics
        let sql = sql.as_ref();

        log::debug!(
            "{}({}): {}",
            "query_iter".green().bold(),
            "Txn".purple(),
            sql
        );
        let error = fail!("select row: {}", sql);

        let mut stmt = self
            .txn
            .prepare_cached(sql)
            .context(fail!("prepare sql: {}", sql))?;
        let mut rows = stmt
            .query(rsq::params_from_iter(params))
            .context(fail!("query_iter: {}", sql))?;

        let mut v = Vec::<T>::new();
        while let Some(row) = rows.next().context(fail!("get next item"))? {
            v.push(f(row));
        }

        Ok(v)
    }

    /// Shorter function that allows for querying the database from an
    /// [`SqlBuilder`]. Calls `query_iter` internally
    ///
    /// `query_iter`: Txn::query_iter
    pub(crate) fn query_builder<T, F>(&self, builder: &SqlBuilder, f: F) -> Result<Vec<T>>
    where
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        self.query_iter(builder.utf()?, builder.params_as_slice(), f)
    }

    /// Shorter function that allows for querying the database from an
    /// [`SqlBuilder`]. Calls `query_vec` internally on a named parameters
    /// vector, i.e., `&[(&str, &dyn ToSql)]`
    ///
    /// `query_vec`: Txn::query_vec
    pub(crate) fn query_named_builder<T, F>(&self, builder: &SqlBuilder, f: F) -> Result<Vec<T>>
    where
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        self.query_vec(builder.utf()?, builder.named_params_as_slice(), f)
    }

    // ============================== Version =============================
    // ====================================================================

    // TODO: Remove these if not used

    /// Insert the latest version into the database
    pub(crate) fn insert_version(&self) -> Result<()> {
        let v = Version::build().context("failed to get current version")?;

        self.insert(
            "INSERT INTO version (major, minor, patch)
                VALUES (?1, ?2, ?3)",
            params![v.major(), v.minor(), v.patch()],
        )
        .context("failed to insert version into `version` table")?;

        Ok(())
    }

    /// Get the current version of the database
    pub(crate) fn get_current_version(&self) -> Result<Version> {
        let res = self
            .select("SELECT * from version", params![], |row| {
                Ok(Version::new(row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .context("failed to query row")?;

        Ok(res)
    }

    /// Update the current version of the database
    pub(crate) fn update_current_version(&self) -> Result<()> {
        let v = Version::build().context("failed to get current version")?;

        self.execute(
            "UPDATE version SET major = ?1, minor = ?2, patch = ?3",
            params![v.major(), v.minor(), v.patch()],
        )
        .context("failed to update current version")?;

        Ok(())
    }

    // ========================= Tracking Actions =========================
    // ====================================================================

    /// Insert a change into the `tracker` table, which keeps tracks of actions
    /// modifying anything in the entire database
    pub(crate) fn record_modification<S: AsRef<str>>(
        &self,
        table: &Table,
        op: &ModType,
        id: ID,
        previous: S,
    ) -> Result<usize> {
        self.execute(
            "INSERT INTO tracker (table, operation, operation_id, previous)
            VALUES (?1, ?2, ?3, ?4)",
            params![table, op, id, previous.as_ref()],
        )
        .context("failed to record a change")
    }

    /// Create a checkpoint in the `checkpoint` table
    pub(crate) fn create_checkpoint<S: AsRef<str>>(&self, desc: S) -> Result<usize> {
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
            params![desc.as_ref()],
        )
        .context("failed to create `checkpoint`")
    }
}
