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
use crate::{fail, query_fail, retr_fail, wutag_fatal};
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
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of `Value`s in the database
    pub(super) fn value_count(&self) -> Result<u32> {
        self.select1::<u32>(
            "SELECT count(1)
            FROM value",
        )
        .context(retr_fail!("`Value` count"))
    }

    /// Retrieve the number of tags a given [`Value`] is associated with
    pub(super) fn value_count_by_id(&self, id: ValueId) -> Result<u32> {
        let count: u32 = self
            .select(
                "SELECT count(value_id)
                FROM file_tag
                WHERE value_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .context("failed to query file_tag for value id count")?;

        Ok(count)
    }

    /// Retrieve all `Value`s in the database
    pub(super) fn values(&self) -> Result<Values> {
        let values: Vec<Value> = self
            .query_vec(
                "SELECT id, name
                FROM value
                ORDER BY name",
                params![],
                |row| row.try_into().expect("failed to convert to `Value`"),
            )
            .context(query_fail!("`Values`"))?;

        Ok(values.into())
    }

    /// Retrieve the `Value` matching the `ValueId` in the database
    pub(super) fn value(&self, vid: ValueId) -> Result<Value> {
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
            .context(query_fail!("`Value`"))?;

        Ok(value)
    }

    /// Retrieve all `Value`s matching the vector of `ValueId`s
    pub(super) fn values_by_valueids(&self, ids: Vec<ValueId>) -> Result<Values, Error> {
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
            .context(query_fail!("`Values`"))?;

        Ok(values.into())
    }

    // TEST:
    /// Retrieve all unused `Value`s within the database
    pub(super) fn values_unused(&self) -> Result<Values> {
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
            .context(query_fail!("`Value`"))?;

        Ok(values.into())
    }

    /// Retrieve a `Value` by its string name
    ///   - **Exact match** searching
    pub(super) fn value_by_name<S: AsRef<str>>(&self, name: S, ignore_case: bool) -> Result<Value> {
        let mut builder = SqlBuilder::new_initial(
            "SELECT id, name
            FROM value
            WHERE name ",
        );
        builder.nocase_collation(ignore_case);
        builder.append("= ?1");

        let value: Value = self
            .select(&(builder.utf()?), params![name.as_ref()], |row| {
                let r: Value = row.try_into().expect("failed to convert to `Value`");
                Ok(r)
            })
            .context(query_fail!("`Value`"))?;

        Ok(value)
    }

    /// Retrieve all `Value`s matching a vector of names
    ///   - **Exact match** searching
    pub(super) fn values_by_names<S: AsRef<str>>(
        &self,
        names: &[S],
        ignore_case: bool,
    ) -> Result<Values, Error> {
        if names.is_empty() {
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
            builder.append_param(name.as_ref().to_owned());
        }

        builder.append(")");
        builder.nocase_collation(ignore_case);

        let values: Vec<Value> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Value`")
            })
            .context(query_fail!("`Values`"))?;

        Ok(values.into())
    }

    /// Retrieve all [`Value`]s matching a [`TagId`]
    pub(super) fn values_by_tagid(&self, tid: TagId) -> Result<Values> {
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
            .context("failed to query for values by TagId")?;

        Ok(values.into())
    }

    // ============================== Pattern =============================
    // ====================================================================

    /// Query for files using a custom function
    fn select_values_by_func(&self, func: &str, reg: &str) -> Result<Values> {
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
            .context(query_fail!("`Value`", "pattern", reg))?;

        Ok(values.into())
    }

    /// Query for [`Values`] using a the `pcre` regex custom function
    pub(super) fn select_values_by_pcre(&self, reg: &str) -> Result<Values> {
        self.select_values_by_func("pcre", reg)
    }

    /// Query for [`Values`] using a the `regex` custom function
    pub(super) fn select_values_by_regex(&self, reg: &str) -> Result<Values> {
        self.select_values_by_func("regex", reg)
    }

    /// Query for [`Values`] using a the `iregex` custom function
    pub(super) fn select_values_by_iregex(&self, reg: &str) -> Result<Values> {
        self.select_values_by_func("iregex", reg)
    }

    /// Query for [`Values`] using a the `glob` custom function
    pub(super) fn select_values_by_glob(&self, glob: &str) -> Result<Values> {
        self.select_values_by_func("glob", glob)
    }

    /// Query for [`Values`] using a the `iglob` custom function
    pub(super) fn select_values_by_iglob(&self, glob: &str) -> Result<Values> {
        self.select_values_by_func("iglob", glob)
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a `Value` into the database
    pub(super) fn insert_value<S: AsRef<str>>(&self, name: S) -> Result<Value> {
        let name = name.as_ref();
        let res = self.insert(
            "INSERT INTO value (name)
                VALUES (?1)",
            params![name],
        )?;

        Ok(Value::new(ID::new(res), name.to_owned()))
    }

    /// Update the `Value` by changing its' name
    pub(super) fn update_value<S: AsRef<str>>(&self, id: ValueId, new: S) -> Result<Value, Error> {
        let name = new.as_ref();
        let affected = self
            .execute(
                "UPDATE value
                SET name = ?1
                WHERE id = ?2",
                params![name, id],
            )
            .context(fail!("update `Value`"))?;

        if affected == 0 {
            return Err(Error::NonexistentValue(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(Value::new(id, name.to_owned()))
    }

    /// Remove a `Value` from the database
    pub(super) fn delete_value(&self, id: ValueId) -> Result<(), Error> {
        let affected = self
            .execute(
                "DELETE FROM value
                WHERE id = ?1",
                params![id],
            )
            .context(fail!("delete `Value`"))?;

        if affected == 0 {
            return Err(Error::NonexistentFile(id.to_string()));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(id.to_string()));
        }

        Ok(())
    }
}
