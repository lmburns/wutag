//! Contains functions that act on `Txn` to modify the the `impl` table
//!
//! Schema of table:
//! ```sql
//! CREATE TABLE IF NOT EXISTS impl (
//!     tag_id INTEGER NOT NULL,
//!     value_id INTEGER NOT NULL,
//!     implied_tag_id INTEGER NOT NULL,
//!     implied_value_id INTEGER NOT NULL,
//!     PRIMARY KEY (tag_id, value_id, implied_tag_id, implied_value_id)
//! );
//! ```

use super::{
    sqlbuilder::{Sort, SqlBuilder},
    types::{
        file::FileId,
        filetag::{FileTag, FileTags},
        implication::{Implication, Implications},
        tag::{Tag, TagId, TagValueCombo},
        value::{Value, ValueId},
    },
    Error, Txn,
};
use crate::{fail, query_fail};
use anyhow::{Context, Result};
use colored::Colorize;
use std::{convert::TryInto, time::SystemTime};

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Row,
};

// ================================ Txn ===============================
// ============================ Impl Actions ==========================

impl Txn<'_> {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve all [`Implication`]s within the database
    pub(super) fn implications(&self) -> Result<Implications> {
        let impls: Vec<Implication> = self
            .query_vec(
                "SELECT
                  tag.id as 'tag.id',
                  tag.name as 'tag.name',
                  tag.color as 'tag.color',
                  value.id as 'value.id',
                  value.name as 'value.name',
                  implied_tag.id as 'implied_tag.id',
                  implied_tag.name as 'implied_tag.name',
                  implied_tag.color as 'implied_tag.color',
                  implied_value.id as 'implied_value.id',
                  implied_value.name as 'implied_value.name'
              FROM
                impl
                  INNER JOIN tag tag ON impl.tag_id = tag.id
                  LEFT OUTER JOIN value value ON impl.value_id = value.id
                  INNER JOIN tag implied_tag ON impl.implied_tag_id = implied_tag.id
                  LEFT OUTER JOIN value implied_value ON impl.implied_value_id = implied_value.id
              ORDER BY
                  tag.name,
                  value.name,
                  implied_tag.name,
                  implied_value.name",
                params![],
                |row| row.try_into().expect("failed to convert to `Implication`"),
            )
            .context(query_fail!("`Implications`"))?;

        Ok(impls.into())
    }

    /// Retrieve the [`Implication`]s matching the [`TagValueCombo`]s. This
    /// returns the `implied` values
    pub(super) fn implications_for(
        &self,
        tvpairs: &[TagValueCombo],
    ) -> Result<Implications, Error> {
        if tvpairs.is_empty() {
            return Err(Error::EmptyArray);
        }

        let mut builder = SqlBuilder::new();
        builder.append(
            "SELECT
                tag.id as 'tag.id',
                tag.name as 'tag.name',
                tag.color as 'tag.color',
                value.id as 'value.id',
                value.name as 'value.name',
                implied_tag.id as 'implied_tag.id',
                implied_tag.name as 'implied_tag.name',
                implied_tag.color as 'implied_tag.color',
                implied_value.id as 'implied_value.id',
                implied_value.name as 'implied_value.name'
            FROM
              impl
                INNER JOIN tag tag ON impl.tag_id = tag.id
                LEFT OUTER JOIN value value ON impl.value_id = value.id
                INNER JOIN tag implied_tag ON impl.implied_tag_id = implied_tag.id
                LEFT OUTER JOIN value implied_value ON impl.implied_value_id = implied_value.id
            WHERE ",
        );

        for (idx, pair) in tvpairs.iter().enumerate() {
            if idx > 0 {
                builder.append(" OR ");
            }

            builder.append("(impl.tag_id = ");
            builder.append_param(pair.tag_id());
            builder.append(" AND impl.value_id IN (0, ");
            builder.append_param(pair.value_id());
            builder.append("))");
        }

        builder.appendln("ORDER BY tag.name, value.name, implied_tag.name, implied_value.name");

        let impls: Vec<Implication> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Implication`")
            })
            .context(query_fail!("`Implications`"))?;

        Ok(impls.into())
    }

    /// Retrieve the [`Implication`]s matching the [`TagValueCombo`]s. This
    /// returns the `implying` values
    pub(super) fn implications_implying(&self, tvpairs: &[TagValueCombo]) -> Result<Implications> {
        let mut builder = SqlBuilder::new();
        // TODO: Implying here is matching what?
        // `implying` renamed to `implied` to convert to row
        builder.append(
            "SELECT
                tag.id as 'tag.id',
                tag.name as 'tag.name',
                tag.color as 'tag.color',
                value.id as 'value.id',
                value.name as 'value.name',
                implying_tag.id,
                implying_tag.name,
                implying_tag.color,
                implying_value.id,
                implying_value.name,
            FROM
              impl
                INNER JOIN tag tag ON impl.tag_id = tag.id
                LEFT OUTER JOIN value value ON impl.value_id = value.id
                INNER JOIN tag implying_tag ON impl.implying_tag_id = implying_tag.id
                LEFT OUTER JOIN value implying_value ON impl.implying_value_id = implying_value.id
            WHERE ",
        );

        for (idx, pair) in tvpairs.iter().enumerate() {
            if idx > 0 {
                builder.append(" OR ");
            }

            builder.append("(impl.implied_tag_id = ");
            builder.append_param(pair.tag_id());
            builder.append(" AND impl.implied_value_id = ");
            builder.append_param(pair.value_id());
            builder.append(")");
        }

        builder.appendln("ORDER BY tag.name, value.name, implying_tag.name, implying_value.name");

        let impls: Vec<Implication> = self
            .query_builder(&builder, |row| {
                row.try_into().expect("failed to convert to `Implication`")
            })
            .context(query_fail!("`Implications`"))?;

        Ok(impls.into())
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert an [`Implication`] into the `impl` table
    pub(super) fn insert_implication(
        &self,
        pair: &TagValueCombo,
        implied: &TagValueCombo,
    ) -> Result<()> {
        self.insert(
            "INSERT
            OR IGNORE INTO impl (tag_id, value_id, implied_tag_id, implied_value_id)
            VALUES
              (?1, ?2, ?3, ?4)",
            params![
                pair.tag_id(),
                pair.value_id(),
                implied.tag_id(),
                implied.value_id()
            ],
        )
        .context(fail!("`Implication`"))?;

        Ok(())
    }

    /// Remove an [`Implication`] from the `impl` table
    pub(super) fn delete_implication(
        &self,
        pair: &TagValueCombo,
        implied: &TagValueCombo,
    ) -> Result<(), Error> {
        let affected = self
            .execute(
                "DELETE FROM
                  implication
                WHERE
                  tag_id = ?1
                  AND value_id = ?2
                  AND implied_tag_id = ?3
                  AND implied_value_id = ?4",
                params![
                    pair.tag_id(),
                    pair.value_id(),
                    implied.tag_id(),
                    implied.value_id()
                ],
            )
            .context(fail!("`Implication`"))?;

        let e = format!(
            "\nimplying (tag, value): {} {}\nimplied (tag, value): {} {}",
            pair.tag_id(),
            pair.value_id(),
            implied.tag_id(),
            implied.value_id()
        );

        if affected == 0 {
            return Err(Error::NonexistentImpl(e));
        } else if affected > 1 {
            return Err(Error::TooManyChanges(e));
        }

        Ok(())
    }

    /// Remove an [`Implication`] from the `impl` table given a [`TagId`]
    pub(super) fn delete_implication_by_tagid(&self, tid: TagId) -> Result<()> {
        self.execute(
            "DELETE FROM implication
            WHERE tag_id = ?1 OR implied_tag_id = ?1",
            params![tid],
        )
        .context(fail!("`Implication` by `TagId`"))?;

        Ok(())
    }

    /// Remove an [`Implication`] from the `impl` table given a [`ValueId`]
    pub(super) fn delete_implication_by_valueid(&self, vid: ValueId) -> Result<()> {
        self.execute(
            "DELETE FROM implication
            WHERE value_id = ?1 OR implied_value_id = ?1",
            params![vid],
        )
        .context(fail!("`Implication` by `ValueId`"))?;

        Ok(())
    }
}
