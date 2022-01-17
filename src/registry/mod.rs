//! The `Registry` (database) and it's basic commands

// TODO: Remove unused functions
// TODO: Use custom regex and hash functions (add glob)

#![allow(unused)]

pub(crate) mod api;
pub(crate) mod common;
pub(crate) mod file;
pub(crate) mod filetag;
pub(crate) mod implication;
pub(crate) mod querier;
pub(crate) mod query;
pub(crate) mod schema;
pub(crate) mod sqlbuilder;
pub(crate) mod tag;
pub(crate) mod transaction;
pub(crate) mod types;
pub(crate) mod upgrade;
pub(crate) mod value;

use self::{
    common::{
        hash::{blake3_hash, blake3_hash_text},
        version::Version,
    },
    transaction::Txn,
};
use crate::{
    cassert,
    config::Config,
    consts::encrypt::REGISTRY_UMASK,
    directories::PROJECT_DIRS,
    util::{contains_upperchar, prompt},
    wutag_fatal, wutag_info,
};
use anyhow::{Context as _, Result};
use colored::Colorize;
use regex::{Regex, RegexBuilder};
use std::{
    error, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use thiserror::Error;

#[allow(unused_imports)]
use rusqlite::{
    self as rsq,
    functions::{Context, FunctionFlags},
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput},
    Connection, Params, Row, Transaction,
};

/// Name of registry file
pub(crate) const REGISTRY_FILE: &str = "wutag.registry";
/// An error use within the `rusqlite` crate
pub(crate) type BoxError = Box<dyn error::Error + Send + Sync + 'static>;

/// Errors having to do with the `Tag` database/`Registry`
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// There is no `Connection` to the database
    #[error("connection to database does not exist: {0}")]
    NoConnection(PathBuf),

    /// `anyhow` conversion `Error`
    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),

    /// Unable to convert to an integer
    #[error("error converting number: {0}")]
    IntegerConversion(#[from] std::num::TryFromIntError),

    /// There were no matching `Row`s in the database
    #[error("no rows were returned: {0}")]
    NoRows(#[source] rsq::Error),

    /// Unable to close the `Connection`
    #[error("failure to close connection: {0}")]
    CloseConnection(#[source] rsq::Error),

    /// Generalized `Error`
    #[error("{0}")]
    General(String),

    /// `Implication` does not exist
    #[error("implication does not exist in database: {0}")]
    NonexistentImpl(String),

    /// `File` does not exist
    #[error("file does not exist in database: {0}")]
    NonexistentFile(String),

    /// `Value` does not exist
    #[error("value does not exist in database: {0}")]
    NonexistentValue(String),

    /// `Tag` does not exist
    #[error("tag does not exist in database: {0}")]
    NonexistentTag(String),

    /// Too many items were modified
    #[error("too many items were modified with this action: id = {0}")]
    TooManyChanges(String),

    /// No arguments in the array
    #[error("no arguments were given")]
    EmptyArray,
}

// fn param_to_sql(v: &Vec<SqlParam>) -> Vec<&dyn rusqlite::ToSql> {
//     v.iter().map(|p| p as &dyn rusqlite::ToSql).collect()
// }

// ========================== Registry ==========================

/// The `Tag` database
#[derive(Debug)]
pub(crate) struct Registry {
    /// User configuration options
    config:  Config,
    /// Path to the database
    path:    PathBuf,
    /// The open `Connection` for the database
    conn:    Connection,
    /// The version the database is using TODO: Maybe Version struct
    version: u32,
    // -
    // /// Root path of the database
    // root_path: PathBuf,
}

impl Registry {
    /// Return the database path
    pub(crate) const fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Return the database connection
    pub(crate) const fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Return the configuration
    pub(crate) const fn config(&self) -> &Config {
        &self.config
    }

    /// Create a new `Registry`
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn new(config: Config, path: &Path, conn: Connection) -> Result<Self> {
        if !path.exists() {
            wutag_fatal!("database does not exist");
        }

        Ok(Self {
            config,
            path: path.to_path_buf(),
            conn,
            version: 1,
        })
    }

    /// Initialize the database
    pub(crate) fn init(&self) -> Result<()> {
        // Change `atomic commit and rollback` to `Write-Ahead Log`
        self.conn.pragma_update(None, "journal_mode", &"WAL")?;

        // TODO:
        // let version = self
        //     .conn
        //     .prepare("SELECT user_version from pragma_user_version")?
        //     .query_row(params![], |row| row.get::<usize, i64>(0))? as usize;

        // let version: i32 = self
        //     .conn
        //     .pragma_query_value(None, "user_version", |row| row.get(0))?;
        //
        // if version != 0_i32
        //     && version != self.version as i32
        //     && prompt(
        //         "Database version mismatch and it needs to be reset. Is that okay?",
        //         &self.path,
        //     )
        // {
        //     self.clean_db()?;
        // }
        //
        // if version == 0 {
        //     self.conn
        //         .pragma_update(None, "user_version", &self.version)?;
        // }

        self.create_tag_table()?;
        self.create_file_table()?;
        self.create_value_table()?;
        self.create_file_tag_table()?;
        self.create_impl_table()?;
        self.create_query_table()?;
        self.create_version_table()?;
        self.insert_version()?;

        // self.create_tracker_table()?;
        // self.create_checkpoint_table()?;

        self.add_regex_funcs()?;
        self.add_blake3_func()?;

        /// -
        /// -
        let crate_v = Version::build()?;
        let schema_v = self.get_current_version()?;

        if schema_v != crate_v {
            wutag_info!("version mismatch");
        }

        if schema_v.less_than(crate_v) {
            self.recreate_version_table()?;
        }

        self.update_current_version()?;
        /// -
        /// -
        Ok(())
    }

    /// Close the SQL connection
    pub(crate) fn close(self) -> Result<(), Error> {
        self.conn.close().map_err(|e| Error::CloseConnection(e.1))
    }

    // ====================================================================
    // ============================ Executing =============================
    // ====================================================================

    /// Execute a command. Implements the same function as [`execute`]
    ///
    /// [`execute`]: rusqlite::Connection::execute
    pub(crate) fn execute<P>(&self, sql: &str, params: P) -> Result<()>
    where
        P: Params,
    {
        log::debug!(
            "{}({}): {}",
            "execute".green().bold(),
            "Registry".purple(),
            sql
        );

        self.conn
            .execute(sql, params)
            .context(format!("failed to execute command: {}", sql))?;

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
        log::debug!(
            "{}({}): {}",
            "insert".green().bold(),
            "Registry".purple(),
            sql
        );

        let mut stmt = self
            .conn
            .prepare(sql)
            .context(format!("failed to prepare sql: {}", sql))?;

        stmt.insert(params)
            .context(format!("failed to insert item: {}", sql))?;

        Ok(())
    }

    // ====================================================================
    // ============================ Retrieving ============================
    // ====================================================================

    /// Select a row. Implements the same function as [`query_row`]
    ///
    /// [`query_row`]: rusqlite::statement::Statement::query_row
    pub(crate) fn select<F, T, P>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> Result<T, rsq::Error>,
    {
        log::debug!(
            "{}({}): {}",
            "select".green().bold(),
            "Registry".purple(),
            sql
        );
        let error = format!("failed to select row: {}", sql);

        let mut stmt = self
            .conn
            .prepare(sql)
            .context(format!("failed to prepare sql: {}", sql))?;

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
        log::debug!(
            "{}({}): {}",
            "query_vec".green().bold(),
            "Registry".purple(),
            sql
        );

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

    // ============================ Recreation ============================
    // ====================================================================

    /// Remove the database file
    pub(crate) fn clean_db(&self) -> Result<()> {
        fs::remove_file(&self.path).context(format!(
            "failed to remove database: {}",
            self.path.display()
        ))?;

        Ok(())
    }

    /// Recreate the `impl` table
    pub(crate) fn recreate_impl_table(&self) -> Result<()> {
        self.exec_no_params(
            "ALTER TABLE impl
            RENAME TO impl_old",
        )
        .context("failed to alter `impl` table")?;

        self.create_impl_table()?;

        self.exec_no_params(
            "INSERT INTO implication
            SELECT tag_id, 0, implied_tag_id, 0
            FROM impl_old",
        )
        .context("failed to move items from old `impl` to new")?;

        self.exec_no_params("DROP TABLE impl_old")
            .context("failed to drop old `impl` table")?;

        Ok(())
    }

    /// Recreate the `version` table
    pub(crate) fn recreate_version_table(&self) -> Result<()> {
        self.exec_no_params("DROP TABLE version")
            .context("failed to drop `version` table")?;

        self.create_version_table()?;
        self.insert_version()?;

        Ok(())
    }

    // ============================ Conversion ============================
    // ====================================================================

    /// Convert to a [`Txn`](self::transaction::Txn)
    pub(crate) fn txn(&self) -> Result<Txn<'_>> {
        Txn::new(self).context("failed to build `Txn`")
    }

    // ============================== Other ===============================
    // ====================================================================

    // TODO: 1 or 2 args?

    // pub(crate) fn add_recent_function(&self) -> Result<()> {
    //     self.conn
    //         .create_scalar_function(
    //             "recent",
    //             1,
    //             FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
    //             move |ctx| {
    //                 assert_eq!(ctx.len(), 1, "called with unexpected number of
    // arguments");                 let x: String = ctx.get(0)?;
    //                 let x: time::StdDuration =
    //                     duration(&x).map_err(|e|
    // Error::UserFunctionError(e.into()))?;                 let x: Duration =
    //                     Duration::from_std(x).map_err(|e|
    // Error::UserFunctionError(e.into()))?;                 let from =
    // chrono::Local::now() - x;                 Ok(from.to_string())
    //             },
    //         )
    //         .context("failed to create `recent` function")
    // }

    /// Create a regular expression function in the database.
    /// Allow for case-sensitive and case-insensitive functions, as well as
    /// `glob`s
    pub(crate) fn add_regex_func(
        &self,
        fname: &'static str,
        case_insensitive: bool,
        glob: bool,
    ) -> Result<()> {
        self.conn
            .create_scalar_function(
                fname,
                2,
                FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
                move |ctx| {
                    cassert!(
                        ctx.len(),
                        2,
                        "called {} with unexpected number of arguments",
                        fname
                    );
                    let regexp: Arc<Regex> =
                        ctx.get_or_create_aux(0, |vr| -> rsq::Result<_, BoxError> {
                            let s = vr.as_str()?;

                            let patt = if glob {
                                let builder = globset::GlobBuilder::new(s);
                                builder
                                    .build()
                                    .expect("invalid glob sequence")
                                    .regex()
                                    .to_owned()
                            } else {
                                String::from(s)
                            };

                            let reg = RegexBuilder::new(&patt)
                                .case_insensitive(case_insensitive)
                                .build()
                                .map_err(|e| rsq::Error::UserFunctionError(Box::new(e)))?;

                            Ok(reg)
                        })?;

                    let is_match = {
                        let text = ctx
                            .get_raw(1)
                            .as_str()
                            .map_err(|e| rsq::Error::UserFunctionError(e.into()))?;

                        regexp.is_match(text)
                    };

                    Ok(is_match)
                },
            )
            .context("failed to create `regexp` function")
    }

    /// Add regular expression functions to the database.
    ///
    /// Four functions are created:
    ///   - `regex`:  case insensitive: false
    ///   - `iregex`: case insensitive: true
    ///   - `glob`:   case insensitive: false
    ///   - `iglob`:  case insensitive: true
    pub(crate) fn add_regex_funcs(&self) -> Result<()> {
        self.add_regex_func("regex", false, false)
            .context("failed to build `regex` func")?;
        self.add_regex_func("iregex", true, false)
            .context("failed to build `iregex` func")?;
        self.add_regex_func("glob", false, true)
            .context("failed to build `glob` func")?;
        self.add_regex_func("iglob", true, true)
            .context("failed to build `iglob` func")?;

        Ok(())
    }

    /// Add a [`blake3`] hashing function to the database. This hashes the given
    /// string
    pub(crate) fn add_blake3_func(&self) -> Result<()> {
        self.conn
            .create_scalar_function(
                "blake3",
                1,
                FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
                move |ctx| {
                    assert_eq!(ctx.len(), 1, "called with unexpected number of arguments");
                    let text = ctx
                        .get_raw(0)
                        .as_str()
                        .map_err(|e| rsq::Error::UserFunctionError(e.into()))?;

                    Ok(blake3_hash_text(text))
                },
            )
            .context("failed to create `blake3` function")
    }

    // TODO: Maybe delete
    /// Add a [`blake3`] hashing function to the database that hashes a file's
    /// contents
    pub(crate) fn add_blake3_path_func(&self) -> Result<()> {
        self.conn
            .create_scalar_function(
                "blake3",
                1,
                FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
                move |ctx| {
                    assert_eq!(ctx.len(), 1, "called with unexpected number of arguments");
                    let text = ctx
                        .get_raw(0)
                        .as_str()
                        .map_err(|e| rsq::Error::UserFunctionError(e.into()))?;
                    let path = PathBuf::from(text);
                    let mode = path
                        .metadata()
                        .map_err(|_e| rsq::Error::InvalidPath(path))?
                        .permissions()
                        .mode();
                    let hash = blake3_hash(text, mode)
                        .map_err(|e| rsq::Error::UserFunctionError(e.into()))?;

                    Ok(hash.to_string())
                },
            )
            .context("failed to create `blake3` function")
    }
}

// ========================= Helper Functions =========================
// ====================================================================

/// Get the path to the database
pub(crate) fn db_path() -> Result<PathBuf> {
    let data_dir = PROJECT_DIRS.data_dir();

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
    fs::set_permissions(&path, perms).context(format!(
        "failed to set permissions on database: {}",
        path.display()
    ))?;

    Ok(path)
}
