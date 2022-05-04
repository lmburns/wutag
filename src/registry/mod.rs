//! The `Registry` (database) and it's basic commands

// TODO: Remove unused functions
// TODO: Use custom hash function
// TODO: Add ability to detect files in Db but not with xattr
// TODO: Add ability to crawl file system to gain db back

// TODO: Add option to simply print database

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

#[cfg(test)]
mod tests;

use self::{
    common::{
        hash::{blake3_hash, blake3_hash_text},
        version::Version,
    },
    transaction::Txn,
};
use crate::{
    cassert_eq,
    consts::encrypt::REGISTRY_UMASK,
    directories::PROJECT_DIRS,
    util::{contains_upperchar, prompt},
    wutag_error, wutag_fatal, wutag_info,
};
use anyhow::{Context as _, Result};
use colored::Colorize;
use regex::{Regex, RegexBuilder};
use shellexpand::LookupError;
use std::{
    borrow::Cow,
    cmp::Ordering,
    env, error, fmt, fs,
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
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
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

// TODO: Possibly start using 'ON CONFLICT'

/// The `Tag` database
#[derive(Debug)]
pub(crate) struct Registry {
    /// User configuration options
    follow_symlinks: bool,
    /// Path to the database
    path:            PathBuf,
    /// The open [`Connection`] for the database
    conn:            Connection,
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

    /// Return whether symlinks should be followed
    pub(crate) const fn follow_symlinks(&self) -> bool {
        self.follow_symlinks
    }

    /// Create a new [`Registry`]
    pub(crate) fn new<P: AsRef<Path>>(path: Option<P>, follow_symlinks: bool) -> Result<Self> {
        use rusqlite::OpenFlags;
        if let Some(p) = path {
            let path = p.as_ref();
            let registry = &PathBuf::from(
                shellexpand::full(&path.to_string_lossy())
                    .unwrap_or_else(|_| {
                        Cow::from(
                            LookupError {
                                var_name: "Unknown environment variable".into(),
                                cause:    env::VarError::NotPresent,
                            }
                            .to_string(),
                        )
                    })
                    .to_string(),
            );

            // let conn = Connection::open(&registry)?;

            /// SQLITE_OPEN_SHARED_CACHE: shared cache enabled
            /// SQLITE_OPEN_FULL_MUTEX: "serialized" threading mode
            // Others are default
            let conn = Connection::open_with_flags(
                &registry,
                OpenFlags::SQLITE_OPEN_READ_WRITE
                    | OpenFlags::SQLITE_OPEN_CREATE
                    | OpenFlags::SQLITE_OPEN_SHARED_CACHE
                    | OpenFlags::SQLITE_OPEN_FULL_MUTEX
                    | OpenFlags::SQLITE_OPEN_URI,
            )?;

            // The file is created using `Connection`, so set perms after
            self::set_perms(&registry)?;

            Ok(Self {
                follow_symlinks,
                path: registry.clone(),
                conn,
            })
        } else {
            Self::new_default(follow_symlinks)
        }
    }

    /// Create a new [`Registry`] with a default database
    pub(crate) fn new_default(follow_symlinks: bool) -> Result<Self> {
        let state_file = self::db_path()?;

        log::debug!(
            "using default registry: {}",
            state_file.display().to_string().green()
        );
        Self::new(Some(state_file), follow_symlinks)
    }

    /// Close the SQL connection
    pub(crate) fn close(self) -> Result<(), Error> {
        self.conn.close().map_err(|e| Error::CloseConnection(e.1))
    }

    /// Initialize the database. This function has to be ran **every** time a call
    /// is made which may involve a call to a `regex`/`glob` function
    #[rustfmt::skip]
    pub(crate) fn init(&self) -> Result<()> {
        self.conn.busy_timeout(std::time::Duration::from_secs(0))?;
        self.conn.pragma_update(None, "locking_mode", &"exclusive")?;
        self.conn.pragma_update(None, "legacy_file_format", &false)?;
        self.conn.pragma_update(None, "page_size", &4096)?;
        self.conn.pragma_update(None, "cache_size", &(-40 * 1024))?;
        self.conn.pragma_update(None, "threads", (num_cpus::get() * 3) as u32)?;
        self.conn.pragma_update(None, "foreign_keys", &false)?;
        // TODO: Enable once development is done
        // self.conn.pragma_update(None, "journal_mode", &"wal")?;

        // self.conn.pragma_update(None, "synchronous", &"off")?;
        // self.conn.pragma_update(None, "read_uncommitted", &"true")?;
        // self.conn.pragma_update(None, "wal_autocheckpoint", &0u32)?;
        // self.conn.pragma_update(None, "mmap_size", &mmap_size)?;

        self.create_tag_table()?;
        self.create_file_table()?;
        self.create_value_table()?;
        self.create_file_tag_table()?;
        self.create_impl_table()?;
        self.create_query_table()?;
        self.create_version_table()?;

        // TODO:
        // self.create_tracker_table()?;
        // self.create_checkpoint_table()?;

        // Add feature for this function?
        self.add_pcre_function()?;
        self.add_regex_funcs()?;
        self.add_blake3_func()?;
        self.add_fullpath_func()?;

        self.create_unicase_collation()?;

        self.upgrade()?;

        Ok(())
    }

    /// Upgrade the database if it needs to be upgraded
    pub(crate) fn upgrade(&self) -> Result<()> {
        if let Ok(schema_v) = self.get_current_version() {
            log::debug!("checking for correct version");
            let crate_v = Version::build()?;

            if schema_v != crate_v {
                wutag_info!("version mismatch. Database is outdated with wutag's current version");

                if schema_v.less_than(crate_v) {
                    // When updated
                    // self.recreate_version_table()?;
                }
            }
        } else {
            log::debug!("inserting version for the first time");
            self.insert_version()?;
        }

        // self.update_current_version()?;

        Ok(())
    }

    /// Create a collation that is similar to `COLLATE NOCASE`; however, this
    /// works with unicode characters as well
    fn create_unicase_collation(&self) -> Result<()> {
        use unicase::UniCase;

        self.conn
            .create_collation("unicase", |s1, s2| UniCase::new(s1).cmp(&UniCase::new(s2)))
            .context("failed to create `unicase` collation")?;

        Ok(())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Executing                         │
    // ╰──────────────────────────────────────────────────────────╯

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
            .prepare_cached(sql)
            .context(format!("failed to prepare sql: {}", sql))?;

        stmt.insert(params)
            .context(format!("failed to insert item: {}", sql))?;

        Ok(())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Retrieving                        │
    // ╰──────────────────────────────────────────────────────────╯

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
            .prepare_cached(sql)
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
            .prepare_cached(sql)
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

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Recreation                        │
    // ╰──────────────────────────────────────────────────────────╯

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

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Conversion                        │
    // ╰──────────────────────────────────────────────────────────╯

    /// Convert to a [`Txn`]
    pub(crate) fn txn(&self) -> Result<Txn<'_>> {
        Txn::new(self)
    }

    /// Execute a closure on [`Txn`]. This is used to lessen duplicate code
    pub(crate) fn txn_wrap<F, T>(&self, mut f: F) -> Result<T>
    where
        F: FnOnce(&Txn) -> Result<T>,
    {
        f(&self.txn()?)
    }

    /// Execute a closure on [`Txn`] and `commit` by executing the `sqlite`
    /// command `COMMIT` if the closure was successful. Commits only need to be
    /// done if the database is modified
    ///
    /// The `sqlite` command is used instead of the `commit` function due to
    /// shared reference issues
    pub(crate) fn wrap_commit<F, T>(&self, mut f: F) -> Result<T>
    where
        F: FnOnce(&Txn) -> Result<T>,
    {
        self.txn_wrap(|txn| {
            let res = f(txn);

            if res.is_ok() {
                log::debug!("committing");
                println!("===== COMITTING =====");
                txn.registry().conn().execute_batch("COMMIT")?;
            }

            res
        })
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                          Other                           │
    // ╰──────────────────────────────────────────────────────────╯

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

    /// Return a `String` from a user-defined-function
    fn get_string<'a>(
        ctx: &'a Context,
        fname: &'static str,
        idx: usize,
    ) -> Result<&'a str, rsq::Error> {
        ctx.get_raw(idx).as_str().map_err(|e| {
            rsq::Error::UserFunctionError(
                format!("Bad argument at {} to function '{}': {}", idx, fname, e).into(),
            )
        })
    }

    /// Return an optional `String` from a user-defined-function
    fn get_opt_string<'a>(
        ctx: &'a Context,
        fname: &'static str,
        idx: usize,
    ) -> Result<Option<&'a str>> {
        let raw = ctx.get_raw(idx);
        if raw == ValueRef::Null {
            return Ok(None);
        }

        Ok(Some(raw.as_str().map_err(|e| {
            rsq::Error::UserFunctionError(
                format!("Bad argument at {} to function '{}': {}", idx, fname, e).into(),
            )
        })?))
    }

    /// Add a function that is a shorthand for concatenating the file's
    /// directory and its name
    fn add_fullpath_func(&self) -> Result<()> {
        self.conn
            .create_scalar_function(
                "fullpath",
                2,
                FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
                move |ctx| {
                    assert_eq!(ctx.len(), 2, "called with unexpected number of arguments");
                    let dir = Self::get_string(ctx, "fullpath", 0)?;
                    let fname = Self::get_string(ctx, "fullpath", 1)?;

                    Ok(format!("{}/{}", dir, fname))
                },
            )
            .context("failed to create `fullpath` function")
    }

    // TODO: Combine with `Search`
    // TODO: Print miette full capabilities with Glob. It does not work

    /// Create a regular expression function in the database.
    /// Allow for case-sensitive and case-insensitive functions, as well as
    /// `glob`s
    fn add_pattern_func(
        &self,
        fname: &'static str,
        case_insensitive: bool,
        glob: bool,
    ) -> Result<()> {
        use crate::{wutag_error, wutag_fatal};
        use wax::{DiagnosticGlob, DiagnosticResultExt, Glob, GlobError};

        self.conn
            .create_scalar_function(
                fname,
                2,
                FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
                move |ctx| {
                    cassert_eq!(
                        ctx.len(),
                        2,
                        "called {} with unexpected number of arguments",
                        fname
                    );
                    let regexp: Arc<Regex> =
                        ctx.get_or_create_aux(0, |vr| -> rsq::Result<Regex, BoxError> {
                            let s = vr.as_str()?;

                            let patt = if glob {
                                let g = <Glob as DiagnosticGlob>::new(s);
                                // The diagnostics must be printed first
                                // If there are no errors, nothing is printed
                                for diag in g.diagnostics() {
                                    wutag_error!("{}", diag);
                                }

                                g.map_or_else(
                                    |_| std::process::exit(1),
                                    |(glob, _)| glob.regex().to_string(),
                                )
                            } else {
                                String::from(s)
                            };

                            let reg = RegexBuilder::new(&patt)
                                .case_insensitive(case_insensitive)
                                .build()
                                .map_err(|e| rsq::Error::UserFunctionError(Box::new(e)))?;

                            log::debug!("pattern({:#?})", reg);

                            Ok(reg)
                        })?;

                    let matched = {
                        let text = Self::get_string(ctx, fname, 1)?;
                        log::debug!("to match text: {:#?}", text);

                        regexp.is_match(text)
                    };

                    Ok(matched)
                },
            )
            .context(format!("failed to create `{}` function", fname))
    }

    /// Add regular expression functions to the database.
    ///
    /// Four functions are created:
    ///   - `regex`:  case insensitive: false
    ///   - `iregex`: case insensitive: true
    ///   - `glob`:   case insensitive: false
    ///   - `iglob`:  case insensitive: true
    pub(crate) fn add_regex_funcs(&self) -> Result<()> {
        self.add_pattern_func("regex", false, false)
            .context("failed to build `regex` func")?;
        self.add_pattern_func("iregex", true, false)
            .context("failed to build `iregex` func")?;
        self.add_pattern_func("glob", false, true)
            .context("failed to build `glob` func")?;
        self.add_pattern_func("iglob", true, true)
            .context("failed to build `iglob` func")?;

        Ok(())
    }

    /// Add a `pcre` compatible regular expression function to the database
    fn add_pcre_function(&self) -> Result<()> {
        use crate::{wutag_error, wutag_fatal};
        use fancy_regex::{Regex as FancyRegex, RegexBuilder as FancyBuilder};

        self.conn
            .create_scalar_function(
                "pcre",
                2,
                FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
                move |ctx| {
                    assert_eq!(
                        ctx.len(),
                        2,
                        "called pcre with unexpected number of arguments"
                    );
                    let regexp: Arc<FancyRegex> =
                        ctx.get_or_create_aux(0, |vr| -> rsq::Result<FancyRegex, BoxError> {
                            let s = vr.as_str()?;

                            let reg = FancyBuilder::new(s)
                                .build()
                                .map_err(|e| rsq::Error::UserFunctionError(Box::new(e)))?;

                            Ok(reg)
                        })?;

                    let matched = {
                        let text = Self::get_string(ctx, "pcre", 1)?;
                        log::debug!("to match text: {:#?}", text);

                        regexp
                            .is_match(text)
                            .map_err(|e| rsq::Error::UserFunctionError(Box::new(e)))?
                    };

                    Ok(matched)
                },
            )
            .context("failed to create `pcre` function")
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

                    let hash = if let Ok(perm) =
                        path.metadata().map_err(|_e| rsq::Error::InvalidPath(path))
                    {
                        let mode = perm.permissions().mode();
                        blake3_hash(text, Some(mode))
                            .map_err(|e| rsq::Error::UserFunctionError(e.into()))?
                    } else {
                        blake3_hash(text, None)
                            .map_err(|e| rsq::Error::UserFunctionError(e.into()))?
                    };

                    Ok(hash.to_string())
                },
            )
            .context("failed to create `blake3` function")
    }
}

impl fmt::Display for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (v:{})",
            self.path.display().to_string().purple().bold(),
            self.get_current_version()
                .expect("failed to get current version")
                .to_string()
                .green()
        )
    }
}

// ╭──────────────────────────────────────────────────────────╮
// │                     Helper Functions                     │
// ╰──────────────────────────────────────────────────────────╯

/// Get the path to the registry
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
    self::set_perms(path)
}

/// Set permissions on the registry file using [`REGISTRY_UMASK`]
pub(crate) fn set_perms<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(*REGISTRY_UMASK);
    fs::set_permissions(&path, perms).context(format!(
        "failed to set permissions on database: {}",
        path.display()
    ))?;

    Ok(path.to_path_buf())
}
