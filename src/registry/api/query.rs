//! Interactions with the [`Query`] object

use super::super::{
    types::{
        file::FileId,
        implication::{Implication, Implications},
        query::{Queries, Query},
        tag::{TagId, TagValueCombo, TagValueCombos},
        value::ValueId,
    },
    Registry,
};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;

impl Registry {
    // ╒══════════════════════════════════════════════════════════╕
    //                          Retrieving
    // ╘══════════════════════════════════════════════════════════╛

    /// Return all [`Query`] in the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn queries(&self) -> Result<Queries> {
        self.txn_wrap(|txn| txn.queries())
    }

    /// Retrieve a [`Query`] that matches the given text
    pub(crate) fn query<S: AsRef<str>>(&self, q: S) -> Result<Query> {
        self.txn_wrap(|txn| txn.query(q))
    }

    // ╒══════════════════════════════════════════════════════════╕
    //                          Modifying
    // ╘══════════════════════════════════════════════════════════╛

    /// Insert a [`Query`] into the query table. Returns a [`Query`]
    pub(crate) fn insert_query<S: AsRef<str>>(&self, q: S) -> Result<Query> {
        self.wrap_commit(|txn| txn.insert_query(q))
    }

    /// Delete a [`Query`] from the query table
    pub(crate) fn delete_query<S: AsRef<str>>(&self, q: S) -> Result<()> {
        self.wrap_commit(|txn| txn.delete_query(q))
    }
}
