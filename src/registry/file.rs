//! Contains functions that act on `Txn` to modify the `File` object in the
//! `file` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS file (
//!     id INTEGER PRIMARY KEY,
//!     directory TEXT NOT NULL,
//!     name TEXT NOT NULL,
//!     hash TEXT NOT NULL,
//!     mime TEXT NOT NULL,
//!     mtime DATETIME NOT NULL,
//!     ctime DATETIME NOT NULL,
//!     mode INTEGER NOT NULL,
//!     inode INTEGER NOT NULL,
//!     links INTEGER NOT NULL,
//!     uid INTEGER NOT NULL,
//!     gid INTEGER NOT NULL,
//!     size INTEGER NOT NULL,
//!     is_dir BOOLEAN NOT NULL,
//!     is_symlink BOOLEAN NOT NULL,
//!
//!     #[cfg(feature = "file-flags")]
//!     e2pflags INTEGER NOT NULL,
//!
//!     CONSTRAINT con_file_path UNIQUE (directory, name)
//! );
//! CREATE INDEX IF NOT EXISTS idx_file_hash
//! ON file(hash);
//! ```

use super::{
    querier::ast::query::ParsedQuery,
    sqlbuilder::SqlBuilder,
    types::{
        file::{File, FileId, FileIds, Files, MimeType},
        tag::TagId,
        Sort, ID,
    },
    Error, Txn,
};
use crate::{fail, macros::wants_feature_flags, path_str, wutag_fatal};
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use colored::Colorize;
use itertools::Itertools;
use mime::Mime;
use std::{
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    time::SystemTime,
};

use rusqlite::{self as rsq, params};

/// Colorize the error messages. Specifically the file name
macro_rules! cfile {
    ($file:expr) => {
        $file.display().to_string().yellow().bold()
    };
}

/// Colorize the error messages. Other things that are not files
macro_rules! cstr {
    ($file:expr) => {
        $file.to_string().green().bold()
    };
}

/// Return column-name string, with comma based on user feature `file-flags`
pub(crate) fn e2p_feature_comma() -> &'static str {
    wants_feature_flags().then(|| ", e2pflags").unwrap_or("")
}

/// Return column-name string, based on user feature `file-flags`
pub(crate) fn e2p_feature() -> &'static str {
    wants_feature_flags().then(|| "e2pflags").unwrap_or("")
}

// ╒══════════════════════════════════════════════════════════╕
//                             Txn
//                             ---
//                         File Actions
// ╘══════════════════════════════════════════════════════════╛

impl Txn<'_> {
    // ╭──────────────────────────────────────────────────────────╮
    // │                        Retrieving                        │
    // ╰──────────────────────────────────────────────────────────╯

    /// Retrieve the number of [`File`]s in the database
    pub(super) fn select_file_count(&self) -> Result<u32> {
        let debug = "retrieving File count";
        log::debug!("{}", debug);

        self.select1::<u32>(
            "SELECT count(1)
            FROM file",
        )
        .context(fail!("{}", debug))
    }

    /// Select the maximum [`ID`] from [`File`]s
    pub(super) fn select_file_max(&self) -> Result<u32> {
        let debug = "retrieving File max";
        log::debug!("{}", debug);

        let max = self
            .select1::<u32>(
                "SELECT max(id)
                FROM file",
            )
            .context(fail!("{}", debug));

        if max.is_err() {
            return Ok(0);
        }

        max
    }

    /// Retrieve the number of [`File`]s matching a specific `hash`
    pub(super) fn select_file_count_by_hash<S: AsRef<str>>(&self, fp: S) -> Result<u32> {
        let fp = fp.as_ref();
        let debug = format!("retrieving File({}) count by hash", fp);
        log::debug!("{}", debug);

        self.select(
            "SELECT count(id)
            FROM file
            WHERE hash = ?1",
            params![fp],
            |row| row.get(0),
        )
        .context(fail!("{}", debug))
    }

    /// Retrieve all tracked [`Files`] within the database
    pub(super) fn select_files(&self, sort: Option<Sort>) -> Result<Files> {
        let debug = "querying for Files";
        log::debug!("{}", debug);

        let mut builder = SqlBuilder::new();
        builder.append(format!(
            "SELECT
                id,
                directory,
                name,
                hash,
                mime,
                mtime,
                ctime,
                mode,
                inode,
                links,
                uid,
                gid,
                size,
                is_dir,
                is_symlink
                {}
            FROM file",
            e2p_feature_comma()
        ));

        builder.build_sort(sort);

        let files: Vec<File> = self
            .query_vec(builder.utf()?, params![], |row| {
                row.try_into().expect("failed to convert to `File`")
            })
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] that are directories
    pub(super) fn select_directories(&self) -> Result<Files> {
        let debug = "querying for Files by directory";
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE is_dir = true
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// List all [`File`] [`ID`]s
    pub(crate) fn select_ids(&self) -> Result<Vec<ID>> {
        log::debug!("querying for File IDs");

        Ok(self
            .select_files(None)?
            .iter()
            .map(File::id)
            .collect::<Vec<_>>())
    }

    /// Retrieve a specific [`File`] within the database
    pub(super) fn select_file(&self, id: FileId) -> Result<File> {
        let debug = format!("querying for File({})", id);
        log::debug!("{}", debug);

        let file: File = self
            .select(
                &format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE id = ?1",
                    e2p_feature_comma()
                ),
                params![id],
                |row| {
                    let r: File = row.try_into().expect("failed to convert to `File`");
                    Ok(r)
                },
            )
            .context(fail!("{}", debug))?;
        Ok(file)
    }

    /// Retrieve a [`File`] matching a given `directory` and `name` ([`Path`])
    pub(super) fn select_file_by_path<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let path = path.as_ref();
        let debug = format!("querying for File by path {}", path.display());
        log::debug!("{}", debug);

        let file: File = self
            .select(
                &format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE directory = ?1 AND name = ?2",
                    e2p_feature_comma()
                ),
                params![
                    path_str!(path.parent().context(fail!("get parent"))?),
                    path_str!(path.file_name().context(fail!("get file name"))?)
                ],
                |row| {
                    let r: File = row.try_into().expect("failed to convert to `File`");
                    Ok(r)
                },
            )
            .context(fail!("{}", debug))?;
        Ok(file)
    }

    /// Retrieve [`Files`] matching a specific `directory`
    pub(super) fn select_files_by_directory<S: AsRef<str>>(
        &self,
        dir: S,
        cwd: bool,
    ) -> Result<Files> {
        let dir = dir.as_ref();
        let debug = format!("querying for Files by directory {}", dir);
        log::debug!("{}", debug);

        let mut s = format!(
            "SELECT
                id,
                directory,
                name,
                hash,
                mime,
                mtime,
                ctime,
                mode,
                inode,
                links,
                uid,
                gid,
                size,
                is_dir,
                is_symlink
                {}
            FROM file
            WHERE directory = ?1 OR directory LIKE ?2",
            e2p_feature_comma()
        );

        if cwd {
            s = format!("{} {}", s, "OR directory = '.' OR directory LIKE './%");
        }

        s = format!("{} {}", s, "ORDER BY fullpath(directory, name)");

        let files: Vec<File> = self
            .query_vec(&s, params![dir, format!("{}/%", dir)], |row| {
                row.try_into().expect("failed to convert to `File`")
            })
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a specific `hash`
    pub(super) fn select_files_by_hash<S: AsRef<str>>(&self, fp: S) -> Result<Files> {
        let fp = fp.as_ref();
        let debug = format!("querying for Files by hash {}", fp);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE hash = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![fp],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a specific [`MimeType`]
    pub(super) fn select_files_by_mime<S: AsRef<str>>(&self, mime: S) -> Result<Files> {
        let mime = mime.as_ref();
        let debug = format!("querying for Files by mime {}", mime);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE mime = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![mime],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a specific `mtime`
    pub(super) fn select_files_by_mtime<S: AsRef<str>>(&self, mtime: S) -> Result<Files> {
        let mtime = mtime.as_ref();
        let debug = format!("querying for Files by mtime {}", mtime);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE mtime = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![mtime],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a specific `ctime`
    pub(super) fn select_files_by_ctime<S: AsRef<str>>(&self, ctime: S) -> Result<Files> {
        let ctime = ctime.as_ref();
        let debug = format!("querying for Files by ctime {}", ctime);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE ctime = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![ctime],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a specific `mode`
    pub(super) fn select_files_by_mode<S: AsRef<str>>(&self, mode: S) -> Result<Files> {
        let mode = mode.as_ref();
        let debug = format!("querying for Files by mode {}", mode);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE mode = ?1 OR mode = 100 || ?1 OR mode = 10 || ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![mode],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a specific `inode`
    pub(super) fn select_files_by_inode(&self, inode: u64) -> Result<Files> {
        let debug = format!("querying for Files by inode {}", inode);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE inode = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![inode],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a certain number of links
    pub(super) fn select_files_by_links(&self, links: u64) -> Result<Files> {
        let debug = format!("querying for Files by links {}", links);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE links = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![links],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a given `UID`
    pub(super) fn select_files_by_uid(&self, uid: u64) -> Result<Files> {
        let debug = format!("querying for Files by uid {}", uid);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE uid = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![uid],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a given `GID`
    pub(super) fn select_files_by_gid(&self, gid: u64) -> Result<Files> {
        let debug = format!("querying for Files by gid {}", gid);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE gid = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![gid],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Retrieve all [`Files`] matching a specific `size`
    pub(super) fn select_files_by_size(&self, size: u64) -> Result<Files> {
        let debug = format!("querying for Files by size {}", size);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE size = ?1
                ORDER BY fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![size],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    #[cfg(all(
        feature = "file-flags",
        target_family = "unix",
        not(target_os = "macos")
    ))]
    /// Retrieve all [`Files`] matching an `e2p_fileflag`
    pub(super) fn select_files_by_flag<S: AsRef<str>>(&self, given: S) -> Result<Files> {
        let given = given.as_ref();
        let debug = format!("querying for Files by Flags({})", given);
        log::debug!("{}", debug);

        Ok(self
            .select_files(None)?
            .matches(|f| f.e2pflags().has_flags(given)))
    }

    // TODO: MAKE A TEST
    /// Retrieve the set of [`Files`] that are untagged
    pub(super) fn select_files_untagged(&self) -> Result<Files> {
        let debug = "querying for untagged Files";
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE id NOT IN (
                    SELECT distinct(file_id) FROM file_tag
                )",
                    e2p_feature_comma()
                ),
                params![],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    // TODO: ========================================

    /// Retrieve the count of `File`s matching the given `query` and `Path`
    #[allow(clippy::unused_self)]
    pub(crate) fn file_count_by_query<P: AsRef<Path>>(
        &self,
        expr: &ParsedQuery,
        path: P,
        cwd: bool,
        ignore_case: bool,
    ) -> Result<u32> {
        // let debug = format!();
        // log::debug!("{}", debug);
        todo!()
    }

    // /// Retrieve the set of `Files` matching the given `query` and `Path`
    // #[allow(clippy::fn_params_excessive_bools)]
    // pub(crate) fn file_by_query<S: AsRef<str>>(
    //     &self,
    //     expr: S,
    //     path: S,
    //     cwd: bool,
    //     ignore_case: bool,
    //     sort: bool,
    // ) -> Result<Files> {
    //     todo!()
    // }

    // TODO: ========================================

    /// Retrieve the set of `Files` that are duplicates in the database
    pub(crate) fn select_files_duplicates(&self) -> Result<Files> {
        let debug = "querying for duplicate Files";
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE hash IN (
                    SELECT hash
                    FROM file
                    WHERE hash != ''
                    GROUP BY hash
                    HAVING count(1) > 1
                )
                ORDER BY hash, fullpath(directory, name)",
                    e2p_feature_comma()
                ),
                params![],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                     Custom Functions                     │
    // ╰──────────────────────────────────────────────────────────╯

    /// Query for files using a custom function
    fn select_files_by_func(&self, func: &str, column: &str, reg: &str) -> Result<Files> {
        let debug = format!("querying for Files {} {}({})", column, func, reg);
        log::debug!("{}", debug);

        let files: Vec<File> = self
            .query_vec(
                format!(
                    "SELECT
                    id,
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                    {}
                FROM file
                WHERE {}('{}', {}) == 1",
                    e2p_feature_comma(),
                    func,
                    reg,
                    column
                ),
                params![],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context(fail!("{}", debug))?;

        Ok(files.into())
    }

    /// Query for [`Files`] using the `pcre` regex function on any column
    pub(super) fn select_files_by_pcre(&self, column: &str, reg: &str) -> Result<Files> {
        self.select_files_by_func("pcre", column, reg)
    }

    /// Query for [`Files`] using the `regex` custom function on any column
    pub(super) fn select_files_by_regex(&self, column: &str, reg: &str) -> Result<Files> {
        self.select_files_by_func("regex", column, reg)
    }

    /// Query for [`Files`] using the `iregex` custom function on any column
    pub(super) fn select_files_by_iregex(&self, column: &str, reg: &str) -> Result<Files> {
        self.select_files_by_func("iregex", column, reg)
    }

    /// Query for [`Files`] using the `glob` custom function on any column
    pub(super) fn select_files_by_glob(&self, column: &str, glob: &str) -> Result<Files> {
        self.select_files_by_func("glob", column, glob)
    }

    /// Query for [`Files`] using the `iglob` custom function on any column
    pub(super) fn select_files_by_iglob(&self, column: &str, glob: &str) -> Result<Files> {
        self.select_files_by_func("iglob", column, glob)
    }

    /// Query for [`Files`] using the `pcre` custom function on full path
    pub(super) fn select_files_by_pcre_fp(&self, reg: &str) -> Result<Files> {
        self.select_files_by_regex("fullpath(directory, name)", reg)
    }

    /// Query for [`Files`] using the `regex` custom function on full path
    pub(super) fn select_files_by_regex_fp(&self, reg: &str) -> Result<Files> {
        self.select_files_by_regex("fullpath(directory, name)", reg)
    }

    /// Query for [`Files`] using the `iregex` custom function on full path
    pub(super) fn select_files_by_iregex_fp(&self, reg: &str) -> Result<Files> {
        self.select_files_by_iregex("fullpath(directory, name)", reg)
    }

    /// Query for [`Files`] using the `glob` custom function on full path
    pub(super) fn select_files_by_glob_fp(&self, glob: &str) -> Result<Files> {
        self.select_files_by_glob("fullpath(directory, name)", glob)
    }

    /// Query for [`Files`] using the `iglob` custom function on the full path
    pub(super) fn select_files_by_iglob_fp(&self, glob: &str) -> Result<Files> {
        self.select_files_by_iglob("fullpath(directory, name)", glob)
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Modifying                         │
    // ╰──────────────────────────────────────────────────────────╯

    /// Insert a [`File`] into the database
    pub(crate) fn insert_file<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let path = path.as_ref();
        log::debug!("inserting File({})", cfile!(path));

        let mut f = File::new(&path, self.follow_symlinks())?;
        log::trace!("File({}) info:\n{:#?}", cfile!(path), f);

        let id = if wants_feature_flags() {
            self.insert(
                "INSERT INTO file (
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink,
                    e2pflags
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    f.directory(),
                    f.name(),
                    f.hash(),
                    f.mime(),
                    f.mtime(),
                    f.ctime(),
                    f.mode(),
                    f.inode(),
                    f.links(),
                    f.uid(),
                    f.gid(),
                    f.size(),
                    f.is_dir(),
                    f.is_symlink(),
                    f.e2pflags()
                ],
            )?
        } else {
            self.insert(
                "INSERT INTO file (
                    directory,
                    name,
                    hash,
                    mime,
                    mtime,
                    ctime,
                    mode,
                    inode,
                    links,
                    uid,
                    gid,
                    size,
                    is_dir,
                    is_symlink
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    f.directory(),
                    f.name(),
                    f.hash(),
                    f.mime(),
                    f.mtime(),
                    f.ctime(),
                    f.mode(),
                    f.inode(),
                    f.links(),
                    f.uid(),
                    f.gid(),
                    f.size(),
                    f.is_dir(),
                    f.is_symlink(),
                ],
            )?
        };

        f.set_id_mut(ID::new(id));

        Ok(f)
    }

    /// Update a [`File`] that is in the database
    pub(super) fn update_file<P: AsRef<Path>>(&self, id: FileId, path: P) -> Result<File, Error> {
        let path = path.as_ref();
        let debug = format!("updating File({}, {})", id, cfile!(path));
        log::debug!("{}", debug);

        let mut f = File::new(&path, self.follow_symlinks())?;
        log::trace!("File({}) info:\n{:#?}", cfile!(path), f);

        let affected = if wants_feature_flags() {
            self.execute(
                "UPDATE file
                SET
                    directory = ?1,
                    name = ?2,
                    hash = ?3,
                    mime = ?4,
                    mtime = ?5,
                    ctime = ?6,
                    mode = ?7,
                    inode = ?8,
                    links = ?9,
                    uid = ?10,
                    gid = ?11,
                    size = ?12,
                    is_dir = ?13,
                    is_symlink = ?14,
                    e2pflags = ?15
                WHERE id = ?16",
                params![
                    f.directory(),
                    f.name(),
                    f.hash(),
                    f.mime(),
                    f.mtime(),
                    f.ctime(),
                    f.mode(),
                    f.inode(),
                    f.links(),
                    f.uid(),
                    f.gid(),
                    f.size(),
                    f.is_dir(),
                    f.is_symlink(),
                    f.e2pflags(),
                    id
                ],
            )
            .context(fail!("{}", debug))?
        } else {
            self.execute(
                "UPDATE file
                SET
                    directory = ?1,
                    name = ?2,
                    hash = ?3,
                    mime = ?4,
                    mtime = ?5,
                    ctime = ?6,
                    mode = ?7,
                    inode = ?8,
                    links = ?9,
                    uid = ?10,
                    gid = ?11,
                    size = ?12,
                    is_dir = ?13,
                    is_symlink = ?14
                WHERE id = ?15",
                params![
                    f.directory(),
                    f.name(),
                    f.hash(),
                    f.mime(),
                    f.mtime(),
                    f.ctime(),
                    f.mode(),
                    f.inode(),
                    f.links(),
                    f.uid(),
                    f.gid(),
                    f.size(),
                    f.is_dir(),
                    f.is_symlink(),
                    id
                ],
            )
            .context(fail!("{}", debug))?
        };

        if affected == 0 {
            return Err(Error::NonexistentFile(path.to_string_lossy().to_string()));
        }

        f.set_id_mut(id);

        Ok(f)
    }

    /// Delete all [`File`]s from the database
    pub(super) fn clear_files(&self) -> Result<()> {
        let debug = "deleting all Files";
        log::debug!("{}", debug);

        self.exec_no_params("DELETE FROM file")
            .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Remove a [`File`] from the database
    pub(super) fn delete_file(&self, id: FileId) -> Result<(), Error> {
        let debug = format!("deleting File({})", cstr!(id));
        log::debug!("{}", debug);

        let affected = self
            .execute(
                "DELETE FROM file
                WHERE id = ?1",
                params![id],
            )
            .context(fail!("{}", debug))?;

        if affected == 0 {
            return Err(Error::NonexistentFile(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(())
    }

    /// Remove [`File`]s from the database if they are not associated with a
    /// [`Tag`](super::types::tag::Tag)
    pub(super) fn delete_files_untagged(&self, ids: &FileIds) -> Result<()> {
        let debug = format!("deleting untagged Files [{}]", ids.iter().join(","));
        log::debug!("{}", debug);

        for id in ids.iter() {
            self.execute(
                "DELETE FROM file
                WHERE id = ?1
                AND (
                    SELECT count(1)
                     FROM file_tag
                     WHERE file_id = ?1
                ) == 0",
                params![id],
            )
            .context(fail!("{}", debug))?;
        }

        Ok(())
    }
}
