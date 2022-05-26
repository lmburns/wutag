//! Contains functions that act on `Txn` to modify the `FileTag` object in the
//! `filetag` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS file_tag (
//!    file_id INTEGER NOT NULL,
//!    tag_id INTEGER NOT NULL,
//!    value_id INTEGER NOT NULL,
//!    PRIMARY KEY (file_id, tag_id, value_id),
//!    FOREIGN KEY (file_id) REFERENCES file(id),
//!    FOREIGN KEY (tag_id) REFERENCES tag(id),
//!    FOREIGN KEY (value_id) REFERENCES value(id)
//! );
//! CREATE INDEX IF NOT EXISTS idx_file_tag_file_id ON file_tag(file_id);
//! CREATE INDEX IF NOT EXISTS idx_file_tag_tag_id ON file_tag(tag_id);
//! CREATE INDEX IF NOT EXISTS idx_file_tag_value_id ON file_tag(value_id);
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
use crate::fail;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::{convert::TryInto, time::SystemTime};

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Row,
};

// ╒══════════════════════════════════════════════════════════╕
//                             Txn
//                             ---
//                        FileTag Actions
// ╘══════════════════════════════════════════════════════════╛

impl Txn<'_> {
    // ╭──────────────────────────────────────────────────────────╮
    // │                        Retrieving                        │
    // ╰──────────────────────────────────────────────────────────╯

    /// Check whether the given [`File`] has a specified [`Tag`]
    ///
    /// [`File`]: ../types/file/struct.File.html
    /// [`Tag`]: ../types/tag/struct.Tag.html
    pub(super) fn filetag_exists(&self, ft: &FileTag) -> Result<bool, Error> {
        let debug = format!("checking if FileTag({}) exists", ft);
        log::debug!("{}", debug);

        let count: u32 = self
            .select(
                "SELECT count(1)
                FROM file_tag
                WHERE file_id = ?1 AND tag_id = ?2 AND value_id = ?3",
                params![ft.file_id(), ft.tag_id(), ft.value_id()],
                |row| row.get(0),
            )
            .context(fail!("{}", debug))?;

        if count > 0 {
            return Ok(true);
        }

        Ok(false)

        // Err(Error::NonexistentFileTag(
        //     ft.file_id(),
        //     ft.tag_id(),
        //     ft.value_id(),
        // ))
    }

    /// Retrieve the number of `File`-`Tag` pairs in the database
    pub(super) fn select_filetag_count(&self) -> Result<u32> {
        let debug = "retrieving FileTag count";
        log::debug!("{}", debug);

        self.select1::<u32>(
            "SELECT count(1)
            FROM file_tag",
        )
        .context(fail!("{}", debug))
    }

    /// Select all [`Tag`], [`Value`] pair counts
    pub(super) fn select_tag_value_count(&self) -> Result<u32> {
        let debug = "retrieving Tag, Value count";
        log::debug!("{}", debug);

        self.select1::<u32>(
            "SELECT
              count(*)
            FROM
              (
                SELECT
                  tag_id, value_id
                FROM
                  file_tag
                WHERE
                  value_id != 0
                GROUP BY
                  tag_id, value_id
              )",
        )
        .context(fail!("{}", debug))
    }

    /// Retrieve all `File`-`Tag` pairs
    pub(super) fn select_filetags(&self) -> Result<FileTags> {
        let debug = "querying for FileTags";
        log::debug!("{}", debug);

        let filetags: Vec<FileTag> = self
            .query_vec(
                "SELECT file_id, tag_id, value_id
                FROM file_tag",
                params![],
                |row| row.try_into().expect("failed to convert to `FileTag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(filetags.into())
    }

    /// Retrieve the count of `File`-`Tag` pairs for the given [`FileId`]
    pub(super) fn select_filetag_count_by_fileid(&self, fid: FileId) -> Result<u32> {
        let debug = format!("retrieving FileTag count by FileId({})", fid);
        log::debug!("{}", debug);

        self.select(
            "SELECT count(1)
            FROM file_tag
            WHERE file_id = ?1",
            params![fid],
            |row| row.get(0),
        )
        .context(fail!("{}", debug))
    }

    /// Retrieve the count of `File`-`Tag` pairs for the given [`TagId`]
    pub(super) fn select_filetag_count_by_tagid(&self, tid: TagId) -> Result<u32> {
        let debug = format!("retrieving FileTag count by TagId({})", tid);
        log::debug!("{}", debug);

        self.select(
            "SELECT count(1)
            FROM file_tag
            WHERE tag_id = ?1",
            params![tid],
            |row| row.get(0),
        )
        .context(fail!("{}", debug))
    }

    /// Retrieve the count of `File`-`Tag` pairs for the given [`ValueId`]
    pub(super) fn select_filetag_count_by_valueid(&self, vid: ValueId) -> Result<u32> {
        let debug = format!("retrieving FileTag count by ValueId({})", vid);
        log::debug!("{}", debug);

        self.select(
            "SELECT count(1)
            FROM file_tag
            WHERE value_id = ?1",
            params![vid],
            |row| row.get(0),
        )
        .context(fail!("{}", debug))
    }

    /// Retrieve the `File`s that match the [`FileId`]
    pub(super) fn select_filetags_by_fileid(&self, fid: FileId) -> Result<FileTags> {
        let debug = format!("retrieving FileTags by FileId({})", fid);
        log::debug!("{}", debug);

        let filetags: Vec<FileTag> = self
            .query_vec(
                "SELECT file_id, tag_id, value_id
                FROM file_tag
                WHERE file_id = ?1",
                params![fid],
                |row| row.try_into().expect("failed to convert to `FileTag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(filetags.into())
    }

    /// Retrieve the `File`s that match the [`TagId`]
    pub(super) fn select_filetags_by_tagid(&self, tid: TagId) -> Result<FileTags> {
        let debug = format!("retrieving FileTags by TagId({})", tid);
        log::debug!("{}", debug);

        let filetags: Vec<FileTag> = self
            .query_vec(
                "SELECT file_id, tag_id, value_id
                FROM file_tag
                WHERE tag_id = ?1",
                params![tid],
                |row| row.try_into().expect("failed to convert to `FileTag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(filetags.into())
    }

    /// Retrieve the `File`s that match the [`ValueId`]
    pub(super) fn select_filetags_by_valueid(&self, vid: ValueId) -> Result<FileTags> {
        let debug = format!("retrieving FileTags by ValueId({})", vid);
        log::debug!("{}", debug);

        let filetags: Vec<FileTag> = self
            .query_vec(
                "SELECT file_id, tag_id, value_id
                FROM file_tag
                WHERE value_id = ?1",
                params![vid],
                |row| row.try_into().expect("failed to convert to `FileTag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(filetags.into())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Modifying                         │
    // ╰──────────────────────────────────────────────────────────╯

    /// Insert a [`File`]-[`Tag`] pair to the database
    /// Returns the same [`FileTag`] that is passed
    pub(super) fn insert_filetag(&self, ft: &FileTag) -> Result<FileTag> {
        log::debug!("inserting FileTag({})", ft);
        let res = self.insert(
            "INSERT OR IGNORE INTO file_tag (file_id, tag_id, value_id) VALUES (?1, ?2, ?3)",
            params![ft.file_id(), ft.tag_id(), ft.value_id()],
        )?;

        Ok(*ft)
    }

    /// Remove a specified [`FileTag`] from the database
    pub(super) fn delete_filetag(&self, ft: &FileTag) -> Result<(), Error> {
        let debug = format!("deleting FileTag({})", ft);
        log::debug!("{}", debug);

        let affected = self
            .execute(
                "DELETE FROM file_tag
                WHERE file_id = ?1 AND tag_id = ?2 AND value_id = ?3",
                params![ft.file_id(), ft.tag_id(), ft.value_id()],
            )
            .context(fail!("{}", debug))?;

        if affected == 0 {
            return Err(Error::NonexistentFile(ft.file_id().to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(ft.file_id().to_string()));
        }

        Ok(())
    }

    /// Remove a [`FileTag`] from the database that matches the given [`FileId`]
    /// and [`TagId`]
    pub(super) fn delete_filetag_by_fileid_tagid(&self, fid: FileId, tid: TagId) -> Result<()> {
        let debug = format!("deleting FileTag by FileId({}) TagId({})", fid, tid);
        log::debug!("{}", debug);

        self.execute(
            "DELETE FROM file_tag
            WHERE file_id = ?1 and tag_id = ?2",
            params![fid, tid],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Remove a [`FileTag`] from the database that matches the given [`FileId`]
    pub(super) fn delete_filetag_by_fileid(&self, fid: FileId) -> Result<()> {
        let debug = format!("deleting FileTag by FileId({})", fid);
        log::debug!("{}", debug);

        self.execute(
            "DELETE FROM file_tag
            WHERE file_id = ?",
            params![fid],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Remove a [`FileTag`] from the database that matches the given [`TagId`]
    pub(super) fn delete_filetag_by_tagid(&self, tid: TagId) -> Result<()> {
        let debug = format!("deleting FileTag by TagId({})", tid);
        log::debug!("{}", debug);

        self.execute(
            "DELETE FROM file_tag
            WHERE tag_id = ?",
            params![tid],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Remove a [`FileTag`] from the database that matches the given
    /// [`ValueId`]
    pub(super) fn delete_filetag_by_valueid(&self, vid: ValueId) -> Result<()> {
        let debug = format!("deleting FileTag by ValueId({})", vid);
        log::debug!("{}", debug);

        self.execute(
            "DELETE FROM file_tag
            WHERE value_id = ?",
            params![vid],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Modify an existing [`FileTag`], changing its `value_id`
    pub(super) fn update_filetag_valueid(&self, vid: ValueId, fid: FileId) -> Result<()> {
        let debug = format!("updating FileTag by ValueId({}) FileId({})", vid, fid);
        log::debug!("{}", debug);

        self.execute(
            "UPDATE file_tag
            SET value_id = 0
            WHERE value_id = ?1 and file_id = ?2",
            params![vid, fid],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Copy [`FileTag`]s on one `Tag` to another
    pub(super) fn copy_filetags(&self, source_tid: TagId, dest_tid: TagId) -> Result<()> {
        let debug = format!(
            "copying FileTag by TagId({}) => TagId({})",
            source_tid, dest_tid
        );
        log::debug!("{}", debug);

        self.execute(
            "INSERT INTO file_tag (file_id, tag_id, value_id)
            SELECT file_id, ?2, value_id
            FROM file_tag
            WHERE tag_id = ?1",
            params![source_tid, dest_tid],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Copy [`FileTag`]s from one to another
    pub(super) fn copy_filetag_fileid(&self, source_ft: &FileTag, dest_fid: FileId) -> Result<()> {
        let debug = format!(
            "copying FileTag by FileTag({}) => FileId({})",
            source_ft, dest_fid
        );
        log::debug!("{}", debug);

        self.execute(
            "INSERT INTO file_tag (file_id, tag_id, value_id)
            SELECT ?1, tag_id, value_id
            FROM file_tag
            WHERE file_id = ?2 and tag_id = ?3 and value_id = ?4",
            params![
                dest_fid,
                source_ft.file_id(),
                source_ft.tag_id(),
                source_ft.value_id()
            ],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }
}
