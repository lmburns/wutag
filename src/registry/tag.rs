//! Contains functions that act on `Txn` to modify the the `tag` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS tag (
//!     id INTEGER PRIMARY KEY,
//!     name TEXT NOT NULL,
//!     color TEXT NOT NULL
//! );
//! CREATE INDEX IF NOT EXISTS idx_tag_name ON tag(name);
//! ```

use super::{
    sqlbuilder::SqlBuilder,
    types::{
        file::{File, FileId},
        filetag::{FileTag, FileTags},
        tag::{Tag, TagFileCnt, TagId, Tags},
        value::ValueId,
        Sort, ID,
    },
    Error, Txn,
};
use crate::{fail, wutag_error};
use anyhow::{anyhow, Context, Result};
use colored::{Color, Colorize};
use itertools::Itertools;
use std::{convert::TryInto, time::SystemTime};

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Error::SqliteFailure,
    ErrorCode, Row,
};

// ╒══════════════════════════════════════════════════════════╕
//                             Txn
//                             ---
//                         Tag Actions
// ╘══════════════════════════════════════════════════════════╛

impl Txn<'_> {
    // ╭──────────────────────────────────────────────────────────╮
    // │                        Retrieving                        │
    // ╰──────────────────────────────────────────────────────────╯

    /// Retrieve the number of [`Tag`]s within the database
    pub(super) fn select_tag_count(&self) -> Result<u32> {
        let debug = "retrieving Tag count";
        log::debug!("{}", debug);

        self.select1::<u32>(
            "SELECT count(1)
            FROM tag",
        )
        .context(fail!("{}", debug))
    }

    /// Select the maximum [`ID`] from [`Tag`]s
    pub(super) fn select_tag_max(&self) -> Result<u32> {
        let debug = "retrieving Tag max";
        log::debug!("{}", debug);

        let max = self
            .select1::<u32>(
                "SELECT max(id)
                FROM tag",
            )
            .context(fail!("{}", debug));

        if max.is_err() {
            return Ok(0);
        }

        max
    }

    /// Retrieve the number of files a given [`Tag`] is associated with
    pub(super) fn select_tag_count_by_id(&self, id: TagId) -> Result<u32> {
        let debug = format!("retrieving Tag count by TagId({})", id);
        log::debug!("{}", debug);

        let count: u32 = self
            .select(
                "SELECT count(tag_id)
                FROM file_tag
                WHERE tag_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .context(fail!("{}", debug))?;

        Ok(count)
    }

    /// Retrieve all [`Tag`]s within the database
    pub(super) fn select_tags(&self) -> Result<Tags> {
        let debug = "querying for Tags";
        log::debug!("{}", debug);

        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT id, name, color
                FROM tag
                ORDER BY name",
                params![],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Retrieve the [`Tag`] that matches the given [`TagId`]
    pub(super) fn select_tag(&self, id: TagId) -> Result<Tag> {
        let debug = format!("querying for Tag by TagId({})", id);
        log::debug!("{}", debug);

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
            .context(fail!("{}", debug))?;

        Ok(tag)
    }

    /// Select [`Tags`] that are only connected to one [`File`]
    pub(super) fn select_unique_tags_by_file(&self, fid: FileId) -> Result<Tags> {
        let debug = format!("selecting unique Tags for File({})", fid);
        log::debug!("{}", debug);

        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT tag.* FROM tag
                  INNER JOIN (
                    SELECT * FROM
                      (
                        SELECT * FROM file_tag
                        GROUP BY tag_id
                        HAVING count(*) = 1
                      )
                    WHERE file_id = ?1
                  ) AS dt ON tag.id = dt.tag_id",
                params![fid],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Select [`Tags`] that are only connected to one [`File`]
    pub(super) fn select_unique_tags(&self) -> Result<Tags> {
        let debug = "selecting all unique Tags";
        log::debug!("{}", debug);

        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT tag.* FROM tag
                  INNER JOIN (
                    SELECT * FROM file_tag
                    GROUP BY tag_id
                    HAVING count(*) = 1
                  ) AS dt ON tag.id = dt.tag_id",
                params![],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Select all [`Tag`]s that are not associated with a [`Value`] or [`File`]
    pub(super) fn dangling_tags(&self) -> Result<Tags> {
        let debug = "selecting dangling Tags";
        log::debug!("{}", debug);

        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT id, name, color
                 FROM tag
                    WHERE
                      id NOT IN (
                        SELECT
                          DISTINCT(tag_id)
                        FROM
                          file_tag
                      )",
                params![],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Retrieve all [`Tag`]s matching a [`ValueId`]
    pub(super) fn select_tags_by_valueid(&self, vid: ValueId) -> Result<Tags> {
        let debug = format!("querying for Tags b ValueId({})", vid);
        log::debug!("{}", debug);

        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT id, name, color
                FROM tag
                WHERE id IN (
                    SELECT tag_id
                    FROM file_tag
                    WHERE value_id = ?1
                )
                ORDER BY name",
                params![vid],
                |row| row.try_into().expect("failed to convert to Tag"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Retrieve all [`Tag`]s matching a [`FileId`]
    pub(super) fn select_tags_by_fileid(&self, fid: FileId) -> Result<Tags> {
        let debug = format!("querying for Tags by FileId({})", fid);
        log::debug!("{}", debug);

        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT id, name, color
                FROM tag
                WHERE id IN (
                    SELECT tag_id
                    FROM file_tag
                    WHERE file_id = ?1
                )
                ORDER BY name",
                params![fid],
                |row| row.try_into().expect("failed to convert to Tag"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Retrieve all [`Tag`]s matching a [`FileId`] and [`ValueId`]
    pub(super) fn select_tags_by_fileid_valueid(&self, fid: FileId, vid: ValueId) -> Result<Tags> {
        let debug = format!("querying for Tags by FileId({}), ValueId({})", fid, vid);
        log::debug!("{}", debug);
        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT id, name, color
                FROM tag
                WHERE id IN (
                    SELECT tag_id
                    FROM file_tag
                    WHERE file_id = ?1 and value_id = ?2
                )
                ORDER BY name",
                params![fid, vid],
                |row| row.try_into().expect("failed to convert to Tag"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Retrieve all [`Tag`]s that match the vector of [`TagId`]s
    pub(super) fn select_tags_by_ids(&self, ids: &[TagId]) -> Result<Tags, Error> {
        let debug = format!("querying for Tags by TagIds [{}]", ids.iter().join(","));
        log::debug!("{}", debug);

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
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Retrieve the [`Tag`] matching the [`Tag`] name
    ///   - **Exact match** searching
    pub(super) fn select_tag_by_name<S: AsRef<str>>(
        &self,
        name: S,
        ignore_case: bool,
    ) -> Result<Tag> {
        let name = name.as_ref();
        let debug = format!("querying for Tags by name {}", name);
        log::debug!("{}", debug);

        let mut builder = SqlBuilder::new_initial(
            "SELECT id, name, color
            FROM tag
            WHERE name ",
        );

        builder.nocase_collation(ignore_case);
        builder.append("= ?1");

        let tag: Tag = self
            .select(&(builder.utf()?), params![name], |row| {
                let r: Tag = row.try_into().expect("failed to convert to `Tag`");
                Ok(r)
            })
            .context(fail!("{}", debug))?;

        Ok(tag)
    }

    /// Retrieve all [`Tag`]s matching a vector of names
    ///   - **Exact match** searching
    pub(super) fn select_tags_by_names<S: AsRef<str>>(
        &self,
        names: &[S],
        ignore_case: bool,
    ) -> Result<Tags, Error> {
        let names = names
            .iter()
            .map(|n| n.as_ref().to_owned())
            .collect::<Vec<_>>();
        let debug = format!("querying for Tags by names [{}]", names.iter().join(","));
        log::debug!("{}", debug);

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
            builder.append_param(name);
        }

        builder.append(")");
        builder.nocase_collation(ignore_case);

        let tags: Vec<Tag> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Tag`")
            })
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Select a [`File`]'s' [`Tag`]s
    pub(super) fn select_files_tags(&self, file: &File) -> Result<Tags> {
        let debug = format!("querying for Tags by File({})", file.name());
        log::debug!("{}", debug);

        let tags: Vec<Tag> = self
            .query_vec(
                "SELECT *
                     FROM   tag
                     WHERE
                      id IN (
                        SELECT
                          tag_id
                        FROM
                          file_tag
                        WHERE
                          file_id = (
                            SELECT
                              id
                            FROM
                              file
                            WHERE
                              name = ?1
                              AND directory = ?2
                          )
                      )",
                params![file.name(), file.directory()],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Retrieve all [`Tag`]s matching a pattern
    ///   - **Pattern** searching
    fn select_tags_by_func<S: AsRef<str>>(&self, func: S, column: S, patt: S) -> Result<Tags> {
        let func = func.as_ref();
        let patt = patt.as_ref();
        let column = column.as_ref();
        let debug = format!("querying for Tags {} {}({})", column, func, patt);
        log::debug!("{}", debug);

        let tags: Vec<Tag> = self
            .query_vec(
                format!(
                    "SELECT id, name, color
                    FROM tag
                    WHERE {}('{}', {}) == 1",
                    func, patt, column
                ),
                params![],
                |row| row.try_into().expect("failed to convert to `Tag`"),
            )
            .context(fail!("{}", debug))?;

        Ok(tags.into())
    }

    /// Query for tags using the `pcre` custom function on `name`, or `color`
    pub(super) fn select_tags_by_pcre<S: AsRef<str>>(&self, column: S, reg: S) -> Result<Tags> {
        self.select_tags_by_func("pcre", column.as_ref(), reg.as_ref())
    }

    /// Query for tags using the `regex` custom function on `name`, or `color`
    pub(super) fn select_tags_by_regex<S: AsRef<str>>(&self, column: S, reg: S) -> Result<Tags> {
        self.select_tags_by_func("regex", column.as_ref(), reg.as_ref())
    }

    /// Query for tags using the `iregex` custom function on `name`, or `color`
    pub(super) fn select_tags_by_iregex<S: AsRef<str>>(&self, column: S, reg: S) -> Result<Tags> {
        self.select_tags_by_func("iregex", column.as_ref(), reg.as_ref())
    }

    /// Query for files using the `glob` custom function on `name`, or `color`
    pub(super) fn select_tags_by_glob<S: AsRef<str>>(&self, column: S, glob: S) -> Result<Tags> {
        self.select_tags_by_func("glob", column.as_ref(), glob.as_ref())
    }

    /// Query for files using the `iglob` custom function on `name`, or `color`
    pub(super) fn select_tags_by_iglob<S: AsRef<str>>(&self, column: S, glob: S) -> Result<Tags> {
        self.select_tags_by_func("iglob", column.as_ref(), glob.as_ref())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Modifying                         │
    // ╰──────────────────────────────────────────────────────────╯

    /// Insert a [`Tag`] into the database
    pub(super) fn insert_tag<S: AsRef<str>>(&self, name: S, color: Color) -> Result<Tag> {
        let name = name.as_ref();
        log::debug!("inserting Tag({}, {})", name, color.to_fg_str());

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
        let debug = format!("updating name Tag({}) => Tag({})", id, name);
        log::debug!("{}", debug);

        let affected = self
            .execute(
                "UPDATE tag
                SET name = ?1
                WHERE id = ?2",
                params![name, id],
            )
            .context(fail!("{}", debug))?;

        if affected == 0 {
            return Err(Error::NonexistentTag(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(self.select_tag(id).context(fail!("{}", debug))?)
    }

    /// Update the [`Tag`] by changing its' color
    pub(super) fn update_tag_color(&self, id: TagId, color: Color) -> Result<Tag, Error> {
        let debug = format!("updating color Tag({}) => Tag({})", id, color.to_fg_str());
        log::debug!("{}", debug);

        let affected = self
            .execute(
                "UPDATE tag
                SET color = ?1
                WHERE id = ?2",
                params![color.to_fg_str().as_ref().to_string(), id],
            )
            .context(fail!("{}", debug))?;

        if affected == 0 {
            return Err(Error::NonexistentTag(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(self.select_tag(id).context(fail!("{}", debug))?)
    }

    /// Remove a [`Tag`] from the database
    pub(super) fn delete_tag(&self, id: TagId) -> Result<(), Error> {
        let debug = format!("deleting Tag by TagId({})", id);
        log::debug!("{}", debug);

        let affected = self
            .execute(
                "DELETE FROM tag
                WHERE id = ?",
                params![id],
            )
            .context(fail!("{}", debug))?;

        if affected == 0 {
            return Err(Error::NonexistentTag(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(())
    }

    /// Delete a [`File`]'s' [`Tag`]s
    pub(super) fn delete_files_tags(&self, file: &File) -> Result<()> {
        let debug = format!("deleting Tags by File({})", file.name());
        log::debug!("{}", debug);

        self.execute(
            "DELETE FROM tag
             WHERE
              id IN (
                SELECT
                  tag_id
                FROM
                  file_tag
                WHERE
                  file_id = (
                    SELECT
                      id
                    FROM
                      file
                    WHERE
                      name = ?1
                      AND directory = ?2
                  )
              )",
            params![file.name(), file.directory()],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Delete [`Tags`] that are not associated with any [`File`] or [`Tag`]
    pub(super) fn delete_dangling_tags(&self) -> Result<()> {
        let debug = "deleting dangling Tags";
        log::debug!("{}", debug);

        self.execute(
            "DELETE FROM tag
                WHERE
                  id NOT IN (
                    SELECT
                      DISTINCT(tag_id)
                    FROM
                      file_tag
                  )",
            params![],
        )
        .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Delete all [`Tag`]s from the database
    pub(super) fn clear_tags(&self) -> Result<()> {
        let debug = "deleting all Tags";
        log::debug!("{}", debug);

        self.exec_no_params("DELETE FROM tag")
            .context(fail!("{}", debug))?;

        Ok(())
    }

    // BETTER TEST:
    /// Retrieve information about each [`Tag`]. Returns a vector of
    /// [`TagFileCnt`], which contains information about the number of files the
    /// [`Tag`] is associated with
    pub(super) fn tag_information(&self) -> Result<Vec<TagFileCnt>> {
        let debug = "querying for Tag information";
        log::debug!("{}", debug);

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
            .context(fail!("{}", debug))?;

        Ok(tfc)
    }
}
