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
    // ============================ Retrieving ============================
    // ====================================================================

    pub(crate) fn todo_query() {
        todo!()
    }

    // ============================= Modifying ============================
    // ====================================================================
}
