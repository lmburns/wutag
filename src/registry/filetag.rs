//! Contains functions that act on `Txn` to modify the `FileTag` object in the
//! `filetag` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS file_tag (
//!     file_id INTEGER NOT NULL,
//!     tag_id INTEGER NOT NULL,
//!     value_id INTEGER NOT NULL,
//!     PRIMARY KEY (file_id, tag_id, value_id),
//!     FOREIGN KEY (file_id) REFERENCES file(id),
//!     FOREIGN KEY (tag_id) REFERENCES tag(id),
//!     FOREIGN KEY (value_id) REFERENCES value(id)
//! );
//! CREATE INDEX IF NOT EXISTS idx_file_tag_file_id
//! ON file_tag(file_id);
//! CREATE INDEX IF NOT EXISTS idx_file_tag_tag_id
//! ON file_tag(tag_id);
//! CREATE INDEX IF NOT EXISTS idx_file_tag_value_id
//! ON file_tag(value_id);
//! ```

use super::{
    types::{
        file::FileId,
        filetag::{FileTag, FileTags},
        tag::TagId,
        value::ValueId,
    },
    Error, Txn,
};
use crate::{fail, query_fail, retr_fail};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{convert::TryInto, time::SystemTime};

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Row,
};

// ================================ Txn ===============================
// ========================== FileTag Actions =========================

impl Txn<'_> {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Check whether the given `File` has a specified `Tag`
    pub(crate) fn filetag_exists(&self, ft: &FileTag) -> Result<bool> {
        let count: u32 = self
            .select(
                "SELECT count(1)
                FROM file_tag
                WHERE file_id = ?1 AND tag_id = ?2 AND value_id = ?3",
                params![ft.file_id(), ft.tag_id(), ft.value_id()],
                |row| row.get(0),
            )
            .context(fail!("check if `FileTag` exists"))?;

        Ok(count > 0)
    }

    /// Retrieve the number of `File`-`Tag` pairs in the database
    pub(crate) fn select_filetag_count(&self) -> Result<u32> {
        self.select1::<u32>(
            "SELECT count(1)
            FROM file_tag",
        )
        .context("failed to retrieve `FileTag` count")
    }

    /// Retrieve all `File`-`Tag` pairs
    pub(crate) fn select_filetags(&self) -> Result<FileTags> {
        let filetags: Vec<FileTag> = self
            .query_vec(
                "SELECT file_id, tag_id, value_id
                FROM file_tag",
                params![],
                |row| row.try_into().expect("failed to convert to `FileTag`"),
            )
            .context(fail!("query for `FileTags`"))?;

        Ok(filetags.into())
    }

    /// Retrieve the count of `File`-`Tag` pairs for the given `FileId`
    pub(crate) fn select_filetag_count_by_fileid(&self, fid: FileId) -> Result<u32> {
        self.select(
            "SELECT count(1)
            FROM file_tag
            WHERE file_id = ?1",
            params![fid],
            |row| row.get(0),
        )
        .context(retr_fail!("`FileTag` count", "`FileId`"))
    }

    /// Retrieve the count of `File`-`Tag` pairs for the given [`TagId`]
    pub(crate) fn select_filetag_count_by_tagid(&self, tid: TagId) -> Result<u32> {
        self.select(
            "SELECT count(1)
            FROM file_tag
            WHERE tag_id = ?1",
            params![tid],
            |row| row.get(0),
        )
        .context(retr_fail!("`FileTag` count", "`TagId`"))
    }

    /// Retrieve the count of `File`-`Tag` pairs for the given [`ValueId`]
    pub(crate) fn select_filetag_count_by_valueid(&self, vid: ValueId) -> Result<u32> {
        self.select(
            "SELECT count(1)
            FROM file_tag
            WHERE value_id = ?1",
            params![vid],
            |row| row.get(0),
        )
        .context(retr_fail!("`FileTag` count","`ValueId`"))
    }

    /// Retrieve the `File`s that match the `FileId`
    pub(crate) fn select_filetags_by_fileid(&self, fid: FileId) -> Result<FileTags> {
        let filetags: Vec<FileTag> = self
            .query_vec(
                "SELECT file_id, tag_id, value_id
                FROM file_tag
                WHERE file_id = ?1",
                params![fid],
                |row| row.try_into().expect("failed to convert to `FileTag`"),
            )
            .context(query_fail!("`FileTag`"))?;

        Ok(filetags.into())
    }

    /// Retrieve the `File`s that match the [`TagId`]
    pub(crate) fn select_filetags_by_tagid(&self, tid: TagId) -> Result<FileTags> {
        let filetags: Vec<FileTag> = self
            .query_vec(
                "SELECT file_id, tag_id, value_id
                FROM file_tag
                WHERE tag_id = ?1",
                params![tid],
                |row| row.try_into().expect("failed to convert to `FileTag`"),
            )
            .context(query_fail!("`FileTag`"))?;

        Ok(filetags.into())
    }

    /// Retrieve the `File`s that match the `ValueId`
    pub(crate) fn select_filetags_by_valueid(&self, vid: ValueId) -> Result<FileTags> {
        let filetags: Vec<FileTag> = self
            .query_vec(
                "SELECT file_id, tag_id, value_id
                FROM file_tag
                WHERE value_id = ?1",
                params![vid],
                |row| row.try_into().expect("failed to convert to `FileTag`"),
            )
            .context(query_fail!("`FileTag`"))?;

        Ok(filetags.into())
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a `File`-`Tag` pair to the database
    /// Returns the same `FileTag` that is passed
    pub(crate) fn insert_filetag(&self, ft: &FileTag) -> Result<FileTag> {
        self.insert(
            "INSERT OR IGNORE INTO file_tag (file_id, tag_id, value_id)
            VALUES (?1, ?2, ?3)",
            params![ft.file_id(), ft.tag_id(), ft.value_id()],
        )
        .context(fail!("insert `FileTag`"))?;

        Ok(*ft)
    }

    /// Remove a [`FileTag`] from the database that matches the given
    /// [`FileTag`]
    pub(crate) fn delete_filetag(&self, ft: &FileTag) -> Result<(), Error> {
        let affected = self
            .execute(
                "DELETE FROM file_tag
                WHERE file_id = ?1 AND tag_id = ?2 AND value_id = ?3",
                params![ft.file_id(), ft.tag_id(), ft.value_id()],
            )
            .context(fail!("delete `FileTag`"))?;

        if affected == 0 {
            return Err(Error::NonexistentFile(ft.file_id().to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(ft.file_id().to_string()));
        }

        Ok(())
    }

    /// Remove a `FileTag` from the database that matches the given `FileId`
    pub(crate) fn delete_filetag_by_fileid(&self, fid: FileId) -> Result<()> {
        self.execute(
            "DELETE FROM file_tag
            WHERE file_id = ?",
            params![fid],
        )
        .context(fail!("delete `FileTag` by `FileId`"))?;

        Ok(())
    }

    /// Remove a `FileTag` from the database that matches the given `TagId`
    pub(crate) fn delete_filetag_by_tagid(&self, tid: TagId) -> Result<()> {
        self.execute(
            "DELETE FROM file_tag
            WHERE tag_id = ?",
            params![tid],
        )
        .context(fail!("delete `FileTag` by `TagId`"))?;

        Ok(())
    }

    /// Remove a `FileTag` from the database that matches the given `ValueId`
    pub(crate) fn delete_filetag_by_valueid(&self, vid: ValueId) -> Result<()> {
        self.execute(
            "DELETE FROM file_tag
            WHERE value_id = ?",
            params![vid],
        )
        .context(fail!("delete `FileTag` by `ValueId`"))?;

        Ok(())
    }

    /// Copy `Tag`s on one `File` to another
    pub(crate) fn copy_filetags(&self, source_tid: TagId, dest_tid: TagId) -> Result<()> {
        self.execute(
            "INSERT INTO file_tag (file_id, tag_id, value_id)
            SELECT file_id, ?2, value_id
            FROM file_tag
            WHERE tag_id = ?1",
            params![source_tid, dest_tid],
        )
        .context(fail!("copy `FileTag`"))?;

        Ok(())
    }
}
