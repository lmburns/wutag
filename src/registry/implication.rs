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
        tag::TagId,
        value::ValueId,
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
// ============================ Impl Actions ==========================

// impl Txn<'_> {
//     // ====================================================================
//     // ============================ Retrieving ============================
//     // ====================================================================
//
//     /// Retrieve all `Implication`s within the database
//     pub(crate) fn implications(&self) -> Result<Implications> {
//         let impls: Vec<Implication> = self.query_vec(
//             "SELECT
//               tag.id as 'tag.id',
//               tag.name as 'tag.name',
//               value.id as 'value.id',
//               value.name as 'value.name',
//               implied_tag.id as 'implied_tag.id',
//               implied_tag.name as 'implied_tag.name',
//               implied_value.id as 'implied_value.id',
//               implied_value.name as 'implied_value.name'
//             FROM
//               impl
//               INNER JOIN tag tag ON impl.tag_id = tag.id
//               LEFT OUTER JOIN value value ON impl.value_id = value.id
//               INNER JOIN tag implied_tag ON impl.implied_tag_id =
// implied_tag.id               LEFT OUTER JOIN value implied_value ON
// impl.implied_value_id = implied_value.id             ORDER BY
//               tag.name,
//               value.name,
//               implied_tag.name,
//               implied_value.name",
//             params![],
//             |row| row.try_into().expect("failed to convert to
// `Implication`"),         );
//
//         Ok(impls.into())
//     }
//
//     // ====================================================================
//     // ============================= Modifying ============================
//     // ====================================================================
// }
