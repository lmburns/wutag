//! Interactions with the [`Tag`] object

use super::super::{
    types::{
        file::FileId,
        implication::{Implication, Implications},
        tag::{Tag, TagId, TagIds, Tags},
        value::ValueId,
    },
    Registry,
};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;

impl Registry {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of [`Tag`]s within the database
    pub(crate) fn tag_count(&self) -> Result<u32> {
        let txn = self.txn()?;
        txn.tag_count()
    }

    /// Retrieve all [`Tag`]s within the database
    pub(crate) fn tags(&self) -> Result<Tags> {
        let txn = self.txn()?;
        txn.tags()
    }

    /// Retrieve the [`Tag`] matching the given [`TagId`]
    pub(crate) fn tag(&self, id: TagId) -> Result<Tag> {
        let txn = self.txn()?;
        txn.tag(id)
    }

    /// Retrieve the [`Tags`] matching the array of [`TagIds`]
    pub(crate) fn tags_by_ids(&self, ids: &TagIds) -> Result<Tags> {
        let txn = self.txn()?;
        Ok(txn.tags_by_ids(ids.inner())?)
    }

    /// Retrieve the [`Tag`] matching the given name
    pub(crate) fn tag_by_name<S: AsRef<str>>(&self, name: S) -> Result<Tag> {
        let txn = self.txn()?;
        txn.tag_by_name(name, false)
    }

    /// Retrieve the [`Tag`] matching the given name (ignoring case)
    pub(crate) fn tag_by_name_nocase<S: AsRef<str>>(&self, name: S) -> Result<Tag> {
        let txn = self.txn()?;
        txn.tag_by_name(name, true)
    }

    /// Retrieve the [`Tags`] matching the given names
    pub(crate) fn tags_by_names<S: AsRef<str>>(&self, names: &[S]) -> Result<Tags> {
        let txn = self.txn()?;
        Ok(txn.tags_by_names(names, false)?)
    }

    /// Retrieve the [`Tags`] matching the given names (ignoring case)
    pub(crate) fn tags_by_names_nocase<S: AsRef<str>>(&self, names: &[S]) -> Result<Tags> {
        let txn = self.txn()?;
        Ok(txn.tags_by_names(names, true)?)
    }

    // ========================= Pattern Matching =========================

    /// Retrieve the [`Tags`] matching the given `regex`
    pub(crate) fn tags_by_regex_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        let txn = self.txn()?;
        txn.select_tags_by_regex("name", patt.as_ref())
    }

    /// Retrieve the [`Tags`] with the name matching the given `iregex`
    pub(crate) fn tags_by_iregex_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        let txn = self.txn()?;
        txn.select_tags_by_regex("name", patt.as_ref())
    }

    /// Retrieve the [`Tags`] with the name matching the given `regex`
    pub(crate) fn tags_by_glob_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        let txn = self.txn()?;
        txn.select_tags_by_glob("name", patt.as_ref())
    }

    /// Retrieve the [`Tags`] with the name matching the given `iglob`
    pub(crate) fn tags_by_iglob_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        let txn = self.txn()?;
        txn.select_tags_by_glob("name", patt.as_ref())
    }

    /// Retrieve the [`Tags`] with the color matching the given `regex`
    pub(crate) fn tags_by_regex_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        let txn = self.txn()?;
        txn.select_tags_by_regex("color", patt.as_ref())
    }

    /// Retrieve the [`Tags`] with the color matching the given `iregex`
    pub(crate) fn tags_by_iregex_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        let txn = self.txn()?;
        txn.select_tags_by_regex("color", patt.as_ref())
    }

    /// Retrieve the [`Tags`] with the color matching the given `regex`
    pub(crate) fn tags_by_glob_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        let txn = self.txn()?;
        txn.select_tags_by_glob("color", patt.as_ref())
    }

    /// Retrieve the [`Tags`] with the color matching the given `iglob`
    pub(crate) fn tags_by_iglob_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        let txn = self.txn()?;
        txn.select_tags_by_glob("color", patt.as_ref())
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a [`Tag`] into the database
    pub(crate) fn insert_tag<S: AsRef<str>>(&self, name: S, color: S) -> Result<Tag> {
        let txn = self.txn()?;

        // TODO: verify color
        txn.insert_tag(name, color)
    }
}
