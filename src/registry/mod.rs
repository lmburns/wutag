//! The `Registry` (database) and it's basic commands

#![allow(unused)]

pub(crate) mod schema;
pub(crate) mod version;

use crate::{consts::encrypt::REGISTRY_UMASK, util::prompt, wutag_fatal};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[allow(unused_imports)]
use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput},
    Connection, Params, Row, Transaction,
};

/// Name of registry file
pub(crate) const REGISTRY_FILE: &str = "wutag.registry";

/// Errors having to do with the `Tag` database/`Registry`
#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("connection to database does not exist: {0}")]
    NoConnection(PathBuf),

    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("error converting number: {0}")]
    IntegerConversion(#[from] std::num::TryFromIntError),

    #[error("no rows were returned: {0}")]
    NoRows(#[source] rsq::Error),

    #[error("{0}")]
    General(String),
}

// fn param_to_sql(v: &Vec<SqlParam>) -> Vec<&dyn rusqlite::ToSql> {
//     v.iter().map(|p| p as &dyn rusqlite::ToSql).collect()
// }

// ===================== Registry =====================

/// The `Tag` database
#[derive(Debug)]
pub(crate) struct Registry<'db> {
    /// Path to the database
    path:    PathBuf,
    /// The open `Connection` for the database
    conn:    &'db Connection,
    /// The version the database is using
    version: u32,
}

impl<'db> Registry<'db> {
    /// Return the database path
    pub(crate) fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Return the database connection
    pub(crate) fn conn(&self) -> &'db Connection {
        self.conn
    }

    /// Create a new `Registry`
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn new(path: &Path, conn: &'db Connection) -> Result<Self> {
        if !path.exists() {
            wutag_fatal!("database does not exist");
        }

        Ok(Self {
            path: path.to_path_buf(),
            conn,
            version: 1,
        })
    }

    /// Initialize the database
    pub(crate) fn init(&self) -> Result<(), Error> {
        // TODO:
        if prompt(
            "Database version mismatch and it needs to be reset. Is that okay?",
            &self.path,
        ) {
            self.clean_db()?;
        }

        self.create_tag_table()?;
        self.create_file_table()?;
        self.create_value_table()?;
        self.create_file_tag_table()?;
        self.create_impl_table()?;
        self.create_query_table()?;
        self.create_version_table()?;

        Ok(())
    }

    /// Remove the database file
    pub(crate) fn clean_db(&self) -> Result<()> {
        fs::remove_file(&self.path)
            .with_context(|| format!("failed to remove database: {}", self.path.display()))?;

        Ok(())
    }

    // /// Wrapper-type function:
    // ///     Calls a function on the `Connection` if it is valid
    // pub(crate) fn if_conn<F, T>(&self, f: F) -> Result<T>
    // where
    //     F: Fn(&'db Connection) -> Result<T>,
    // {
    //     if let Some(conn) = self.connection {
    //         f(conn).context("failed to call function on `Connection`")
    //     } else {
    //         return Err(Error::NoConnection(self.path.clone()).into());
    //     }
    // }

    // ===================== Executing ====================

    /// Execute a command. Implements the same function as [`execute`]
    ///
    /// [`execute`]: rusqlite::Connection::execute
    pub(crate) fn execute<P>(&self, sql: &str, params: P) -> Result<()>
    where
        P: Params,
    {
        log::debug!("{}(sql): {}", "execute".green().bold(), sql);

        self.conn
            .execute(sql, params)
            .with_context(|| format!("failed to execute command: {}", sql))?;

        Ok(())
    }

    /// Execute a command with no [`params`]. Implements the same function as
    /// [`execute`]
    ///
    /// [`execute`]: rusqlite::Connection::execute
    /// [`params`]: rusqlite::params
    pub(crate) fn exec_no_params(&self, sql: &str) -> Result<()> {
        self.execute(sql, params![])?;

        Ok(())
    }

    /// Insert a row into the database. Implements the same function as
    /// [`insert`]
    /// This function may not be necessary and [`execute`] can be used instead
    ///
    /// [`insert`]: rusqlite::statement::Statement::insert
    /// [`execute`]: crate::registry::registry::Registry::execute
    pub(crate) fn insert<P>(&self, sql: &str, params: P) -> Result<()>
    where
        P: Params,
    {
        log::debug!("{}(sql): {}", "insert".green().bold(), sql);

        let mut stmt = self
            .conn
            .prepare(sql)
            .context(format!("failed to prepare sql: {}", sql))?;

        stmt.insert(params)
            .context(format!("failed to insert item: {}", sql))?;

        Ok(())
    }

    // ===================== Retrieving ===================

    /// Select a row. Implements the same function as [`query_row`]
    ///
    /// [`query_row`]: rusqlite::statement::Statement::query_row
    pub(crate) fn select<F, T, P>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> Result<T, rsq::Error>,
    {
        log::debug!("{}(sql): {}", "select".green().bold(), sql);
        let error = format!("failed to select row: {}", sql);

        let mut stmt = self
            .conn
            .prepare(sql)
            .with_context(|| format!("failed to prepare sql: {}", sql))?;

        stmt.query_row(params, f).context(error)
    }

    /// Select all matching rows. Implements a function on each of these
    /// matches, returning a vector of type `T`. Implements [`query`] but is
    /// closer in equivalency to [`query_map`]
    ///
    /// [`query`]: rusqlite::statement::Statement::query
    /// [`query_map`]: rusqlite::statement::Statement::query_map
    pub(crate) fn query_vec<F, T, P>(&self, sql: &str, params: P, f: F) -> Result<Vec<T>>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> T + Copy,
    {
        log::debug!("{}(sql): {}", "query_vec".green().bold(), sql);

        let mut stmt = self
            .conn
            .prepare(sql)
            .context(format!("failed to prepare sql: {}", sql))?;
        let mut rows = stmt
            .query(params)
            .context(format!("failed to query row(s): {}", sql))?;

        let mut v = Vec::<T>::new();
        while let Some(row) = rows.next().context("failed to get next item")? {
            v.push(f(row));
        }

        Ok(v)
    }

    // ======================== Other =====================

    // pub(crate) fn commit(&self) -> Result<()> {
    //     log::debug!("{}", "commit".green().bold());
    //     let error = "failed to commit database";
    //
    //     self.if_conn(|conn| {
    //         conn.com
    //     }).context(error)?;
    //
    //     Ok(())
    // }
}

/// Get the path to the database
pub(crate) fn db_path() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    let data_dir_og = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| dirs::home_dir().map(|d| d.join(".local").join("share")))
        .context("Invalid data directory");

    #[cfg(not(target_os = "macos"))]
    let data_dir_og = dirs::data_local_dir();

    let data_dir = data_dir_og
        .map(|p| p.join("wutag"))
        .context("unable to join registry path")?;

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).unwrap_or_else(|_| {
            wutag_fatal!(
                "unable to create tag registry directory: {}",
                data_dir.display()
            )
        });
    }

    let path = data_dir.join(REGISTRY_FILE);

    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(*REGISTRY_UMASK);
    fs::set_permissions(&path, perms)
        .with_context(|| format!("failed to set permissions on database: {}", path.display()))?;

    Ok(path)
}
