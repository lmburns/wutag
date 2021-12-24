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
use anyhow::{Context, Result};
use colored::Colorize;
use std::{convert::TryInto, time::SystemTime};

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Row,
};

// ================================ Txn ===============================
// ============================ Tag Actions ===========================

impl Txn<'_> {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of [`Tag`]s within the database
    pub(crate) fn tag_count(&self) -> Result<u32> {
        self.select1::<u32>(
            "SELECT count(1)
            FROM tag",
        )
        .context("failed to retrieve `Tag` count")
    }

    /// Retrieve all [`Tag`]s within the database
    pub(crate) fn tags(&self) -> Result<Tags> {
        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT id, name, color
                FROM tag
                ORDER BY name",
                params![],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context("failed to query for `Tags`")?;

        Ok(tags.into())
    }

    /// Retrieve the [`Tag`] that matches the given [`TagId`]
    pub(crate) fn tag(&self, id: TagId) -> Result<Tag> {
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
            .context("failed to query for single `Tag`")?;

        Ok(tag)
    }

    /// Retrieve all [`Tag`]s that match the vector of [`TagId`]s
    pub(crate) fn tags_by_ids(&self, ids: Vec<TagId>) -> Result<Tags, Error> {
        if ids.is_empty() {
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new();
        builder.append(
            "SELECT id, name, color
            FROM tag
            WHERE id IN (",
        );

        for id in ids {
            builder.append_param(id);
        }

        builder.append(")");

        let tags: Vec<Tag> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Tag`")
            })
            .context("failed to query for `Tags`")?;

        Ok(tags.into())
    }

    /// Retrieve the [`Tag`] matching the [`Tag`] name
    ///   - **Exact match** searching
    pub(crate) fn tag_by_name<S: AsRef<str>>(&self, name: S, ignore_case: bool) -> Result<Tag> {
        let mut builder = SqlBuilder::new();
        builder.append(
            "SELECT id, name, color
            FROM tag
            WHERE name ",
        );
        builder.nocase_collation(ignore_case);
        builder.append("?1");

        let tag: Tag = self
            .select(&(builder.utf()?), params![name.as_ref()], |row| {
                let r: Tag = row.try_into().expect("failed to convert to `Tag`");
                Ok(r)
            })
            .context("failed to query for `Tag`")?;

        Ok(tag)
    }

    /// Retrieve all [`Tag`]s matching a vector of names
    ///   - **Exact match** searching
    pub(crate) fn tags_by_names(
        &self,
        names: Vec<String>,
        ignore_case: bool,
    ) -> Result<Tags, Error> {
        if names.is_empty() {
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new();
        builder.append(
            "SELECT id, name, color
            FROM tag
            WHERE name ",
        );
        builder.nocase_collation(ignore_case);
        builder.append(" IN (");

        for name in names {
            builder.append_param(name);
        }

        builder.append(")");
        builder.nocase_collation(ignore_case);

        let tags: Vec<Tag> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Tag`")
            })
            .context("failed to query for `Tags`")?;

        Ok(tags.into())
    }

    // ============================= Modifying ============================
    // ====================================================================

    // TODO: Does a tag object really need to be returned?

    /// Insert a [`Tag`] into the database
    pub(crate) fn insert_tag<S: AsRef<str>>(&self, name: S, color: S) -> Result<Tag> {
        let name = name.as_ref();
        let color = color.as_ref();
        let res = self
            .insert(
                "INSERT INTO tag (name, color)
                VALUES (?1, ?2)",
                params![name, color],
            )
            .context("failed to insert `Tag`")?;

        Ok(Tag::new(ID::new(res), name, color))
    }

    /// Update the [`Tag`] by changing its' name
    pub(crate) fn update_tag_name<S: AsRef<str>>(&self, id: TagId, new: S) -> Result<Tag, Error> {
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

        Ok(self
            .tag(id)
            .context(format!("failed to get tag with id: {}", id))?)
    }

    /// Update the [`Tag`] by changing its' color
    pub(crate) fn update_tag_color<S: AsRef<str>>(
        &self,
        id: TagId,
        color: S,
    ) -> Result<Tag, Error> {
        let color = color.as_ref();
        let affected = self
            .execute(
                "UPDATE tag
                SET color = ?1
                WHERE id = ?2",
                params![color, id],
            )
            .context("failed to update `Tag` by color")?;

        if affected == 0 {
            return Err(Error::NonexistentTag(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(self
            .tag(id)
            .context(format!("failed to get tag with id: {}", id))?)
    }

    /// Remove a [`Tag`] from the database
    pub(crate) fn delete_tag(&self, id: TagId) -> Result<(), Error> {
        let affected = self
            .execute(
                "DELETE FROM tag
                WHERE id = ?",
                params![id],
            )
            .context("failed to delete `Tag`")?;

        if affected == 0 {
            return Err(Error::NonexistentTag(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(())
    }

    /// Retrieve information about each [`Tag`]. Returns a vector of
    /// [`TagFileCnt`], which contains information about the number of files the
    /// [`Tag`] is associated with
    pub(crate) fn tag_information(&self) -> Result<Vec<TagFileCnt>> {
        let tfc: Vec<TagFileCnt> = self
            .query_vec(
                "SELECT t.id, t.name, count(file_id) as cnt
                FROM file_tag ft, tag t
                WHERE ft.tag_id = t.id
                GROUP BY t.id
                ORDER BY t.name",
                params![],
                |row| row.try_into().expect("failed to convert to `TagFileCnt`"),
            )
            .context("failed to get `Tag` information in `TagFileCnt`")?;

        Ok(tfc)
    }
}
