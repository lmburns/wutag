//! Contains functions that act on `Txn` to modify the the `value` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS value (
//!   id INTEGER PRIMARY KEY,
//!   name TEXT NOT NULL,
//!   CONSTRAINT con_value_name UNIQUE (name)
//! );
//! ```

use super::{
    sqlbuilder::SqlBuilder,
    types::{
        file::FileId,
        tag::TagId,
        value::{Value, ValueId, Values},
        Sort, ID,
    },
    Error, Txn,
};
use crate::fail;
use anyhow::{Context, Result};
use colored::Colorize;
use itertools::Itertools;
use std::convert::TryInto;

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Row,
};

// ╒══════════════════════════════════════════════════════════╕
//                             Txn
//                             ---
//                        Value Actions
// ╘══════════════════════════════════════════════════════════╛

impl Txn<'_> {
    // ╭──────────────────────────────────────────────────────────╮
    // │                        Retrieving                        │
    // ╰──────────────────────────────────────────────────────────╯

    /// Retrieve the number of [`Value`]s in the database
    pub(super) fn select_value_count(&self) -> Result<u32> {
        let debug = "retrieving Value count";
        log::debug!("{}", debug);

        self.select1::<u32>(
            "SELECT count(1)
            FROM value",
        )
        .context(fail!("{}", debug))
    }

    /// Retrieve the number of tags a given [`Value`] is associated with
    pub(super) fn select_value_count_by_id(&self, id: ValueId) -> Result<u32> {
        let debug = format!("retrieving Value({}) count", id);
        log::debug!("{}", debug);

        let count: u32 = self
            .select(
                "SELECT count(value_id)
                FROM file_tag
                WHERE value_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .context(fail!("{}", debug))?;

        Ok(count)
    }

    /// Select the maximum [`ID`] from [`Value`]s
    pub(super) fn select_value_max(&self) -> Result<u32> {
        let debug = "retrieving Value max";
        log::debug!("{}", debug);

        let max = self
            .select1::<u32>(
                "SELECT max(id)
                FROM value",
            )
            .context(fail!("{}", debug));

        if max.is_err() {
            return Ok(0);
        }

        max
    }

    /// Retrieve all [`Value`]s in the database
    pub(super) fn select_values(&self) -> Result<Values> {
        let debug = "retrieving Values";
        log::debug!("{}", debug);

        let values: Vec<Value> = self
            .query_vec(
                "SELECT id, name
                FROM value
                ORDER BY name",
                params![],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context(fail!("{}", debug))?;

        Ok(values.into())
    }

    /// Retrieve the [`Value`] matching the [`ValueId`] in the database
    pub(super) fn select_value(&self, vid: ValueId) -> Result<Value> {
        let debug = format!("querying Value({})", vid);
        log::debug!("{}", debug);

        let value: Value = self
            .select(
                "SELECT id, name
                FROM value
                WHERE id = ?1",
                params![vid],
                |row| {
                    let r: Value = row.try_into().expect("failed to convert to `Value`");
                    Ok(r)
                },
            )
            .context(fail!("{}", debug))?;

        Ok(value)
    }

    /// Retrieve all [`Value`]s matching the vector of [`ValueId`]s
    pub(super) fn select_values_by_valueids(&self, ids: Vec<ValueId>) -> Result<Values, Error> {
        let debug = format!("querying for Value by ID [{}]", ids.iter().join(","));
        log::debug!("{}", debug);
        if ids.is_empty() {
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new_initial(
            "SELECT id, name
            FROM value
            WHERE id IN (",
        );

        for id in ids {
            builder.append_param(id);
        }

        builder.append(")");

        let values: Vec<Value> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Value`")
            })
            .context(fail!("{}", debug))?;

        Ok(values.into())
    }

    // TEST:
    /// Retrieve all unused [`Value`]s within the database
    pub(super) fn select_values_unused(&self) -> Result<Values> {
        let debug = "querying for unused Values";
        log::debug!("{}", debug);

        let values: Vec<Value> = self
            .query_vec(
                "SELECT id, name FROM value
                WHERE id NOT IN (
                    SELECT
                      distinct(value_id)
                    FROM
                      file_tag
                  )",
                params![],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context(fail!("{}", debug))?;

        Ok(values.into())
    }

    /// Retrieve a [`Value`] by its string name
    ///   - **Exact match** searching
    pub(super) fn select_value_by_name<S: AsRef<str>>(
        &self,
        name: S,
        ignore_case: bool,
    ) -> Result<Value> {
        let name = name.as_ref();
        let debug = format!("querying for Value({})", name);
        log::debug!("{}", debug);

        let mut builder = SqlBuilder::new_initial(
            "SELECT id, name
            FROM value
            WHERE name ",
        );
        builder.nocase_collation(ignore_case);
        builder.append("= ?1");

        let value: Value = self
            .select(&(builder.utf()?), params![name], |row| {
                let r: Value = row.try_into().expect("failed to convert to `Value`");
                Ok(r)
            })
            .context(fail!("{}", debug))?;

        Ok(value)
    }

    /// Retrieve all [`Value`]s matching a vector of names
    ///   - **Exact match** searching
    pub(super) fn select_values_by_names<S: AsRef<str>>(
        &self,
        names: &[S],
        ignore_case: bool,
    ) -> Result<Values, Error> {
        let names = names
            .iter()
            .map(|n| n.as_ref().to_owned())
            .collect::<Vec<_>>();

        let debug = format!("querying for Value by names [{}]", names.iter().join(","));
        log::debug!("{}", debug);

        if names.is_empty() {
            log::debug!("names are empty when querying for Value");
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new_initial(
            "SELECT id, name
            FROM value
            WHERE name ",
        );

        builder.nocase_collation(ignore_case);
        builder.append(" IN (");

        for name in names {
            builder.append_param(name);
        }

        builder.append(")");
        builder.nocase_collation(ignore_case);

        let values: Vec<Value> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Value`")
            })
            .context(fail!("{}", debug))?;

        Ok(values.into())
    }

    /// Retrieve all [`Value`]s matching a [`TagId`]
    pub(super) fn select_values_by_tagid(&self, tid: TagId) -> Result<Values> {
        let debug = format!("querying for Value by TagId({})", tid);
        log::debug!("{}", debug);

        let values: Vec<Value> = self
            .query_vec(
                "SELECT id, name
                FROM value
                WHERE id IN (
                    SELECT value_id
                    FROM file_tag
                    WHERE tag_id = ?1
                )
                ORDER BY name",
                params![tid],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context(fail!("{}", debug))?;

        Ok(values.into())
    }

    /// Retrieve all [`Value`]s matching a [`FileId`]
    pub(super) fn select_values_by_fileid(&self, fid: FileId) -> Result<Values> {
        let debug = format!("querying for Values by FileId({})", fid);
        log::debug!("{}", debug);

        let values: Vec<Value> = self
            .query_vec(
                "SELECT id, name
                FROM value
                WHERE id IN (
                    SELECT value_id
                    FROM file_tag
                    WHERE file_id = ?1
                )
                ORDER BY name",
                params![fid],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context(fail!("{}", debug))?;

        Ok(values.into())
    }

    /// Retrieve all [`Value`]s matching a [`FileId`] and [`TagId`]
    pub(super) fn select_values_by_fileid_tagid(&self, fid: FileId, tid: TagId) -> Result<Values> {
        let debug = format!("querying for Values by FileId({}), TagId({})", fid, tid);
        log::debug!("{}", debug);

        let values: Vec<Value> = self
            .query_vec(
                "SELECT id, name
                FROM value
                WHERE id IN (
                    SELECT value_id
                    FROM file_tag
                    WHERE file_id = ?1 and tag_id = ?2
                )
                ORDER BY name",
                params![fid, tid],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context(fail!("{}", debug))?;

        Ok(values.into())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                         Pattern                          │
    // ╰──────────────────────────────────────────────────────────╯

    /// Query for files using a custom function
    fn select_values_by_func(&self, func: &str, reg: &str) -> Result<Values> {
        let debug = format!("querying for Value {}({})", func, reg);
        log::debug!("{}", debug);

        let values: Vec<Value> = self
            .query_vec(
                format!(
                    "SELECT id, name,
                    FROM value
                    WHERE {}('{}', name) == 1",
                    func, reg,
                ),
                params![],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context(fail!("{}", debug))?;

        Ok(values.into())
    }

    /// Query for [`Values`] using the `pcre` regex custom function
    pub(super) fn select_values_by_pcre(&self, reg: &str) -> Result<Values> {
        self.select_values_by_func("pcre", reg)
    }

    /// Query for [`Values`] using the `regex` custom function
    pub(super) fn select_values_by_regex(&self, reg: &str) -> Result<Values> {
        self.select_values_by_func("regex", reg)
    }

    /// Query for [`Values`] using the `iregex` custom function
    pub(super) fn select_values_by_iregex(&self, reg: &str) -> Result<Values> {
        self.select_values_by_func("iregex", reg)
    }

    /// Query for [`Values`] using the `glob` custom function
    pub(super) fn select_values_by_glob(&self, glob: &str) -> Result<Values> {
        self.select_values_by_func("glob", glob)
    }

    /// Query for [`Values`] using the `iglob` custom function
    pub(super) fn select_values_by_iglob(&self, glob: &str) -> Result<Values> {
        self.select_values_by_func("iglob", glob)
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a [`Value`] into the database
    pub(super) fn insert_value<S: AsRef<str>>(&self, name: S) -> Result<Value> {
        let name = name.as_ref();
        log::debug!("inserting Value({})", name);

        let res = self.insert(
            "INSERT INTO value (name)
                VALUES (?1)",
            params![name],
        )?;

        Ok(Value::new(ID::new(res), name.to_owned()))
    }

    /// Update the [`Value`] by changing its' name
    pub(super) fn update_value<S: AsRef<str>>(&self, id: ValueId, new: S) -> Result<Value, Error> {
        let name = new.as_ref();
        let debug = format!("updating Value({}) => Value({})", id, new.as_ref());
        log::debug!("{}", debug);

        let affected = self
            .execute(
                "UPDATE value
                SET name = ?1
                WHERE id = ?2",
                params![name, id],
            )
            .context(fail!("{}", debug))?;

        if affected == 0 {
            return Err(Error::NonexistentValue(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(Value::new(id, name.to_owned()))
    }

    /// Delete all [`Value`]s from the database
    pub(super) fn clear_values(&self) -> Result<()> {
        let debug = "deleting all Values";
        log::debug!("{}", debug);

        self.exec_no_params("DELETE FROM value")
            .context(fail!("{}", debug))?;

        Ok(())
    }

    /// Remove a [`Value`] from the database
    pub(super) fn delete_value(&self, id: ValueId) -> Result<(), Error> {
        let debug = format!("deleting Value({})", id);
        log::debug!("{}", debug);

        let affected = self
            .execute(
                "DELETE FROM value
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
}
