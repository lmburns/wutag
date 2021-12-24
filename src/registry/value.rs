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
    sqlbuilder::{Sort, SqlBuilder},
    types::{
        file::FileId,
        tag::TagId,
        value::{Value, ValueId, Values},
        ID,
    },
    Error, Txn,
};
use crate::wutag_fatal;
use anyhow::{Context, Result};
use colored::Colorize;
use std::convert::TryInto;

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Row,
};

// ================================ Txn ===============================
// =========================== Value Actions ==========================

impl Txn<'_> {
    // ====================================================================
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of `Value`s in the database
    pub(crate) fn value_count(&self) -> Result<u32> {
        self.select1::<u32>(
            "SELECT id, name
            FROM value
            ORDER BY name",
        )
        .context("failed to retrieve `Value` count")
    }

    /// Retrieve all `Value`s in the database
    pub(crate) fn values(&self) -> Result<Values> {
        let values: Vec<Value> = self
            .query_vec(
                "SELECT id, name
                FROM value
                ORDER BY name",
                params![],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context("failed to query for `Values`")?;

        Ok(values.into())
    }

    /// Retrieve the `Value` matching the `ValueId` in the database
    pub(crate) fn value(&self, vid: ValueId) -> Result<Value> {
        let value: Value = self
            .select(
                "SELECT id, name
                FROM value
                WHERE id = ?",
                params![vid],
                |row| {
                    let r: Value = row.try_into().expect("failed to convert to `Value`");
                    Ok(r)
                },
            )
            .context("failed to query for `Value`")?;

        Ok(value)
    }

    /// Retrieve all `Value`s matching the vector of `ValueId`s
    pub(crate) fn values_by_valueids(&self, ids: Vec<ValueId>) -> Result<Values, Error> {
        if ids.is_empty() {
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new();
        builder.append(
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
            .context("failed to query for `Values`")?;

        Ok(values.into())
    }

    /// Retrieve all unused `Value`s within the database
    pub(crate) fn values_unused(&self) -> Result<Values> {
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
            .context("failed to query for `Value`")?;

        Ok(values.into())
    }

    /// Retrieve a `Value` by its string name
    ///   - **Exact match** searching
    pub(crate) fn value_by_name<S: AsRef<str>>(&self, name: S, ignore_case: bool) -> Result<Value> {
        let mut builder = SqlBuilder::new();
        builder.append(
            "SELECT id, name
            FROM value
            WHERE name ",
        );
        builder.nocase_collation(ignore_case);
        builder.append("?1");

        let value: Value = self
            .select(&(builder.utf()?), params![name.as_ref()], |row| {
                let r: Value = row.try_into().expect("failed to convert to `Value`");
                Ok(r)
            })
            .context("failed to query for `Value`")?;

        Ok(value)
    }

    /// Retrieve all `Value`s matching a vector of names
    ///   - **Exact match** searching
    pub(crate) fn values_by_names(
        &self,
        names: Vec<String>,
        ignore_case: bool,
    ) -> Result<Values, Error> {
        if names.is_empty() {
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new();
        builder.append(
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
            .context("failed to query for `Values`")?;

        Ok(values.into())
    }

    /// Retrieve all [`Value`]s matching a `TagId`
    pub(crate) fn values_by_tagid(&self, tid: TagId) -> Result<Values> {
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
                params![],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context("failed to query `Values`")?;

        Ok(values.into())
    }

    // ====================================================================
    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a `Value` into the database
    pub(crate) fn insert_value<S: AsRef<str>>(&self, name: S) -> Result<Value> {
        let name = name.as_ref();
        let res = self
            .insert(
                "INSERT INTO value (name)
                VALUES (?1)",
                params![name],
            )
            .context("failed to insert `Value`")?;

        Ok(Value::new(ID::new(res), name.to_owned()))
    }

    /// Update the `Value` by changing its' name
    pub(crate) fn update_value<S: AsRef<str>>(&self, id: ValueId, new: S) -> Result<Value, Error> {
        let name = new.as_ref();
        let affected = self
            .execute(
                "UPDATE value
                SET name = ?1
                WHERE id = ?2",
                params![name, id],
            )
            .context("failed to update `Value`")?;

        if affected == 0 {
            return Err(Error::NonexistentValue(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(Value::new(id, name.to_owned()))
    }

    /// Remove a `Value` from the database
    pub(crate) fn delete_value(&self, id: ValueId) -> Result<(), Error> {
        let affected = self
            .execute(
                "DELETE FROM value
                WHERE id = ?1",
                params![id],
            )
            .context("failed to delete `Value`")?;

        if affected == 0 {
            return Err(Error::NonexistentFile(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(())
    }
}
