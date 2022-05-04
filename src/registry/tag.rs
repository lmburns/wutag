//! Contains functions that act on `Txn` to modify the the `tag` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS tag (
//!     id INTEGER PRIMARY KEY,
//!     name TEXT NOT NULL,
//!     color TEXT NOT NULL
//! );
//! CREATE INDEX IF NOT EXISTS idx_tag_name
//! ON tag(name);
//! ```

use super::{
    sqlbuilder::{Sort, SqlBuilder},
    types::{
        file::FileId,
        filetag::{FileTag, FileTags},
        implication::{Implication, Implications},
        tag::{Tag, TagFileCnt, TagId, Tags},
        value::ValueId,
        ID,
    },
    Error, Txn,
};
use crate::{conv_fail, fail, failure, query_fail, retr_fail, wutag_error};
use anyhow::{anyhow, Context, Result};
use colored::{Color, Colorize};
use std::{convert::TryInto, time::SystemTime};

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Error::SqliteFailure,
    ErrorCode, Row,
};

// ================================ Txn ===============================
// ============================ Tag Actions ===========================

impl Txn<'_> {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of [`Tag`]s within the database
    pub(super) fn tag_count(&self) -> Result<u32> {
        self.select1::<u32>(
            "SELECT count(1)
            FROM tag",
        )
        .context(retr_fail!("`Tag` count"))
    }

    /// Retrieve all [`Tag`]s within the database
    pub(super) fn tags(&self) -> Result<Tags> {
        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT id, name, color
                FROM tag
                ORDER BY name",
                params![],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context(query_fail!("`Tag`"))?;

        Ok(tags.into())
    }

    /// Retrieve the [`Tag`] that matches the given [`TagId`]
    pub(super) fn tag(&self, id: TagId) -> Result<Tag> {
        let tag: Tag = self
            .select(
                "SELECT id, name, color
                FROM tag
                WHERE id = ?1",
                params![id],
                |row| {
                    let r: Tag = row.try_into().expect("failed to convert to `Tag`");
                    Ok(r)
                },
            )
            .context(query_fail!("single `Tag`"))?;

        Ok(tag)
    }

    /// Retrieve all [`Tag`]s that match the vector of [`TagId`]s
    pub(super) fn tags_by_ids(&self, ids: &[TagId]) -> Result<Tags, Error> {
        if ids.is_empty() {
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new_initial(
            "SELECT id, name, color
            FROM tag
            WHERE id IN (",
        );

        for id in ids {
            builder.append_param(id.to_string());
        }

        builder.append(")");

        let tags: Vec<Tag> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Tag`")
            })
            .context(query_fail!("`Tags`"))?;

        Ok(tags.into())
    }

    /// Retrieve the [`Tag`] matching the [`Tag`] name
    ///   - **Exact match** searching
    pub(super) fn tag_by_name<S: AsRef<str>>(&self, name: S, ignore_case: bool) -> Result<Tag> {
        let mut builder = SqlBuilder::new_initial(
            "SELECT id, name, color
            FROM tag
            WHERE name ",
        );

        builder.nocase_collation(ignore_case);
        builder.append("= ?1");

        let tag: Tag = self
            .select(&(builder.utf()?), params![name.as_ref()], |row| {
                let r: Tag = row.try_into().expect("failed to convert to `Tag`");
                Ok(r)
            })
            .context(query_fail!("`Tag`"))?;

        Ok(tag)
    }

    /// Retrieve all [`Tag`]s matching a vector of names
    ///   - **Exact match** searching
    pub(super) fn tags_by_names<S: AsRef<str>>(
        &self,
        names: &[S],
        ignore_case: bool,
    ) -> Result<Tags, Error> {
        if names.is_empty() {
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new_initial(
            "SELECT id, name, color
            FROM tag
            WHERE name ",
        );

        builder.nocase_collation(ignore_case);
        builder.append(" IN (");

        for name in names {
            builder.append_param(name.as_ref().to_string());
        }

        builder.append(")");
        builder.nocase_collation(ignore_case);

        let tags: Vec<Tag> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Tag`")
            })
            .context(query_fail!("`Tags`"))?;

        Ok(tags.into())
    }

    /// Retrieve all [`Tag`]s matching a pattern
    ///   - **Pattern** searching
    fn select_tags_by_func<S: AsRef<str>>(&self, func: S, column: S, patt: S) -> Result<Tags> {
        let tags: Vec<Tag> = self
            .query_vec(
                format!(
                    "SELECT id, name, color
                    FROM tag
                    WHERE {}('{}', {}) == 1",
                    func.as_ref(),
                    patt.as_ref(),
                    column.as_ref()
                ),
                params![],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context(query_fail!("`Tags`"))?;

        Ok(tags.into())
    }

    /// Query for tags using a the `pcre` custom function on `name`, or `color`
    pub(super) fn select_tags_by_pcre<S: AsRef<str>>(&self, column: S, reg: S) -> Result<Tags> {
        self.select_tags_by_func("pcre", column.as_ref(), reg.as_ref())
    }

    /// Query for tags using a the `regex` custom function on `name`, or `color`
    pub(super) fn select_tags_by_regex<S: AsRef<str>>(&self, column: S, reg: S) -> Result<Tags> {
        self.select_tags_by_func("regex", column.as_ref(), reg.as_ref())
    }

    /// Query for tags using a the `iregex` custom function on `name`, or
    /// `color`
    pub(super) fn select_tags_by_iregex<S: AsRef<str>>(&self, column: S, reg: S) -> Result<Tags> {
        self.select_tags_by_func("iregex", column.as_ref(), reg.as_ref())
    }

    /// Query for files using a the `glob` custom function on `name`, or `color`
    pub(super) fn select_tags_by_glob<S: AsRef<str>>(&self, column: S, glob: S) -> Result<Tags> {
        self.select_tags_by_func("glob", column.as_ref(), glob.as_ref())
    }

    /// Query for files using a the `iglob` custom function on `name`, or
    /// `color`
    pub(super) fn select_tags_by_iglob<S: AsRef<str>>(&self, column: S, glob: S) -> Result<Tags> {
        self.select_tags_by_func("iglob", column.as_ref(), glob.as_ref())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Modifying                         │
    // ╰──────────────────────────────────────────────────────────╯

    /// Insert a [`Tag`] into the database
    pub(super) fn insert_tag<S: AsRef<str>>(&self, name: S, color: Color) -> Result<Tag> {
        let name = name.as_ref();

        let res = self.insert(
            "INSERT INTO tag (name, color)
                VALUES (?1, ?2)",
            params![name, color.to_fg_str().as_ref().to_string()],
        )?;

        Ok(Tag::new(ID::new(res), name, color))
    }

    /// Update the [`Tag`] by changing its' name
    pub(super) fn update_tag_name<S: AsRef<str>>(&self, id: TagId, new: S) -> Result<Tag, Error> {
        let name = new.as_ref();
        let affected = self
            .execute(
                "UPDATE tag
                SET name = ?1
                WHERE id = ?2",
                params![name, id],
            )
            .context("failed to update `Tag` by name")?;

        if affected == 0 {
            return Err(Error::NonexistentTag(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(self.tag(id).context(retr_fail!("tag", "id", id))?)
    }

    /// Update the [`Tag`] by changing its' color
    pub(super) fn update_tag_color(&self, id: TagId, color: Color) -> Result<Tag, Error> {
        let affected = self
            .execute(
                "UPDATE tag
                SET color = ?1
                WHERE id = ?2",
                params![color.to_fg_str().as_ref().to_string(), id],
            )
            .context("failed to update `Tag` by color")?;

        if affected == 0 {
            return Err(Error::NonexistentTag(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(self.tag(id).context(retr_fail!("tag", "id", id))?)
    }

    /// Remove a [`Tag`] from the database
    pub(super) fn delete_tag(&self, id: TagId) -> Result<(), Error> {
        let affected = self
            .execute(
                "DELETE FROM tag
                WHERE id = ?",
                params![id],
            )
            .context(fail!("delete `Tag`"))?;

        if affected == 0 {
            return Err(Error::NonexistentTag(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(())
    }

    // BETTER TEST:
    /// Retrieve information about each [`Tag`]. Returns a vector of
    /// [`TagFileCnt`], which contains information about the number of files the
    /// [`Tag`] is associated with
    pub(super) fn tag_information(&self) -> Result<Vec<TagFileCnt>> {
        let tfc: Vec<TagFileCnt> = self
            .query_vec(
                "SELECT t.id, t.name, COUNT(file_id) as cnt
                FROM file_tag ft, tag t
                WHERE ft.tag_id = t.id
                GROUP BY t.id
                ORDER BY t.name",
                params![],
                |row| row.try_into().expect("failed to convert to `TagFileCnt`"),
            )
            .context(retr_fail!("`Tag` information", "`TagFileCnt`"))?;

        Ok(tfc)
    }
}
