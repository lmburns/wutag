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
//!     size INTEGER NOT NULL,
//!     is_dir BOOLEAN NOT NULL,
//!     CONSTRAINT con_file_path UNIQUE (directory, name)
//! );
//! CREATE INDEX IF NOT EXISTS idx_file_hash
//! ON file(hash);
//! ```

use super::{
    common::hash::blake3_hash,
    sqlbuilder::{Sort, SqlBuilder},
    types::{
        file::{File, FileId, Files, MimeType},
        ID,
    },
    Error, Txn,
};
use crate::{path_str, wutag_fatal};
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
// =========================== File Actions ===========================

impl Txn<'_> {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of [`File`]s in the database
    pub(crate) fn file_count(&self) -> Result<u32> {
        self.select1::<u32>(
            "SELECT count(1)
            FROM file",
        )
        .context("failed to retrieve `File` count")
    }

    /// Retrieve the number of [`File`]s matching a specific `hash`
    pub(crate) fn file_count_by_hash<S: AsRef<str>>(&self, fp: S) -> Result<u32> {
        self.select(
            "SELECT count(id)
            FROM file
            WHERE hash = ?1",
            params![fp.as_ref()],
            |row| row.get(0),
        )
        .context("failed to retrieve `File` count by hash")
    }

    /// Retrieve all tracked [`File]s within the database
    pub(crate) fn files(&self, sort: Option<Sort>) -> Result<Files> {
        let mut builder = SqlBuilder::new();
        builder.append(
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
                is_dir
            FROM file",
        );

        if let Some(s) = sort {
            builder.appendln(s.to_string());
        }

        let files: Vec<File> = self
            .query_vec(builder.utf()?, params![], |row| {
                row.try_into().expect("failed to convert to `File`")
            })
            .context("failed to query for `File`")?;

        Ok(files.into())
    }

    /// Retrieve a specific [`File`] within the database
    pub(crate) fn file(&self, id: FileId) -> Result<File> {
        let file: File = self
            .select(
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
                    is_dir
                FROM file
                WHERE id = ?1",
                params![id],
                |row| {
                    let r: File = row.try_into().expect("failed to convert to `File`");
                    Ok(r)
                },
            )
            .context("failed to query for single `File`")?;
        Ok(file)
    }

    /// Retrieve a [`File`] matching a specified `directory` and `name`
    /// (`PathBuf`)
    pub(crate) fn file_by_path<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let path = path.as_ref();
        // TODO: possibly use bytes
        // let l = path
        //     .parent()
        //     .context("failed to get parent")?
        //     .as_os_str()
        //     .as_bytes();

        let file: File = self
            .select(
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
                    is_dir
                FROM file
                WHERE directory = ?1 AND name = ?2",
                params![
                    path_str!(path.parent().context("failed to get parent")?),
                    path_str!(path.file_name().context("failed to get file name")?)
                ],
                |row| {
                    let r: File = row.try_into().expect("failed to convert to `File`");
                    Ok(r)
                },
            )
            .context("failed to query for `File` by path")?;
        Ok(file)
    }

    /// Retrieve all `File`s matching a specific `directory`
    pub(crate) fn files_by_directory<S: AsRef<str>>(&self, dir: S, cwd: bool) -> Result<Files> {
        let dir = dir.as_ref();
        let mut s = String::from(
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
                is_dir
            FROM file
            WHERE directory = ?1 OR directory LIKE ?2",
        );

        if cwd {
            s = format!("{} {}", s, "OR directory = '.' OR directory LIKE './%");
        }

        s = format!("{} {}", s, "ORDER BY directory || '/' || name");

        let files: Vec<File> = self
            .query_vec(&s, params![dir, format!("{}/%", dir)], |row| {
                row.try_into().expect("failed to convert to `File`")
            })
            .context("failed to query for `File` by directory")?;

        Ok(files.into())
    }

    /// Retrieve all [`File`]s matching a specific `hash`
    pub(crate) fn files_by_hash<S: AsRef<str>>(&self, fp: S) -> Result<Files> {
        let fp = fp.as_ref();

        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE hash = ?1
                ORDER BY directory || '/' || name",
                params![fp],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for `File` by")?;

        Ok(files.into())
    }

    /// Retrieve all [`File`]s matching a specific [`MimeType`]
    pub(crate) fn files_by_mime<S: AsRef<str>>(&self, mime: S) -> Result<Files> {
        let mime = mime.as_ref();

        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE mime = ?1
                ORDER BY directory || '/' || name",
                params![mime],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for `File` by mime")?;

        Ok(files.into())
    }

    /// Retrieve all [`File`]s matching a specific `mtime`
    pub(crate) fn files_by_mtime<S: AsRef<str>>(&self, mtime: S) -> Result<Files> {
        let mtime = mtime.as_ref();

        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE mtime = ?1
                ORDER BY directory || '/' || name",
                params![mtime],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for `File` by mtime")?;

        Ok(files.into())
    }

    /// Retrieve all [`File`]s matching a specific `ctime`
    pub(crate) fn files_by_ctime<S: AsRef<str>>(&self, ctime: S) -> Result<Files> {
        let ctime = ctime.as_ref();

        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE ctime = ?1
                ORDER BY directory || '/' || name",
                params![ctime],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for `File` by ctime")?;

        Ok(files.into())
    }

    /// Retrieve all [`File`]s matching a specific `mode`
    pub(crate) fn files_by_mode<S: AsRef<str>>(&self, mode: S) -> Result<Files> {
        let mode = mode.as_ref();

        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE mode = ?1
                ORDER BY directory || '/' || name",
                params![mode],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for `File` by mode")?;

        Ok(files.into())
    }

    /// Retrieve all [`File`]s matching a specific `inode`
    pub(crate) fn files_by_inode(&self, inode: u64) -> Result<Files> {
        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE inode = ?1
                ORDER BY directory || '/' || name",
                params![inode],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for `File` by inode")?;

        Ok(files.into())
    }

    /// Retrieve all [`File`]s matching a specific `size`
    pub(crate) fn files_by_size(&self, size: u64) -> Result<Files> {
        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE size = ?1
                ORDER BY size || directory || '/' || name",
                params![size],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for `File` by size")?;

        Ok(files.into())
    }

    /// Retrieve the set of [`Files`] that are untagged
    pub(crate) fn files_untagged(&self) -> Result<Files> {
        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE id NOT IN (
                    SELECT distinct(file_id) FROM file_tag
                )",
                params![],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for untagged `File`")?;

        Ok(files.into())
    }

    // TODO:

    /// Retrieve the count of `File`s matching the given `query` and `Path`
    #[allow(clippy::unused_self)]
    pub(crate) fn file_count_by_query<S: AsRef<str>, P: AsRef<Path>>(
        &self,
        expr: S,
        path: P,
        cwd: bool,
        explicit: bool,
        ignore_case: bool,
    ) -> Result<u32> {
        todo!()
    }

    // /// Retrieve the set of `Files` matching the given `query` and `Path`
    // #[allow(clippy::fn_params_excessive_bools)]
    // pub(crate) fn file_by_query<S: AsRef<str>>(
    //     &self,
    //     expr: S,
    //     path: S,
    //     cwd: bool,
    //     explicit: bool,
    //     ignore_case: bool,
    //     sort: bool,
    // ) -> Result<Files> {
    //     todo!()
    // }

    /// Retrieve the set of `Files` that are duplicates in the database
    pub(crate) fn files_duplicates(&self) -> Result<Files> {
        let files: Vec<File> = self
            .query_vec(
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
                    is_dir
                FROM file
                WHERE hash IN (
                    SELECT hash
                    FROM file
                    WHERE hash != ''
                    GROUP BY hash
                    HAVING count(1) > 1
                )
                ORDER BY hash, directory || '/' || name",
                params![],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to get duplicate `File`s")?;

        Ok(files.into())
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a `File` into the database
    pub(crate) fn insert_file<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let path = path.as_ref();
        let mut f = File::new(&path, self.registry().follow_symlinks())
            .context("failed to build `File`")?;
        println!("FILE: {:#?}", f);
        let id = self
            .insert(
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
                    is_dir
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    f.directory,
                    f.name,
                    f.hash,
                    f.mime,
                    f.mtime,
                    f.ctime,
                    f.mode,
                    f.inode,
                    f.links,
                    f.uid,
                    f.gid,
                    f.size,
                    f.is_dir
                ],
            )
            .context("failed to insert `File`")?;

        f.set_id(ID::new(id));

        Ok(f)
    }

    /// Update a `File` that is already in the database
    pub(crate) fn update_file<P: AsRef<Path>>(&self, id: FileId, path: P) -> Result<File, Error> {
        let path = path.as_ref();
        let mut f = File::new(&path, self.registry().follow_symlinks())
            .context("failed to build `File`")?;

        let affected = self
            .execute(
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
                    is_dir = ?13
                WHERE id = ?14",
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
                    id
                ],
            )
            .context("failed to update `File`")?;

        if affected == 0 {
            return Err(Error::NonexistentFile(path.to_string_lossy().to_string()));
        }

        f.set_id(id);

        Ok(f)
    }

    // TODO: Possibly return `FileId` from every function to log the changes

    /// Remove a `File` from the database
    pub(crate) fn delete_file(&self, id: FileId) -> Result<(), Error> {
        let affected = self
            .execute(
                "DELETE FROM file
                WHERE id = ?1",
                params![id],
            )
            .context("failed to delete `File`")?;

        if affected == 0 {
            return Err(Error::NonexistentFile(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(())
    }

    /// Remove `File`s from the database if they are not associated with a
    /// [`Tag`](super::types::tag::Tag)
    pub(crate) fn delete_file_untagged(&self, ids: Vec<FileId>) -> Result<()> {
        for id in ids {
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
            .context(format!("failed to delete untagged file: {}", id))?;
        }

        Ok(())
    }
}
