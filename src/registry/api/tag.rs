//! Interactions with the [`Tag`] object

use crate::registry::types::tag::TagFileCnt;

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
use colored::{Color, Colorize};
use wutag_core::color::parse_color;

impl Registry {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of [`Tag`]s within the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn tag_count(&self) -> Result<u32> {
        self.txn_wrap(|txn| txn.tag_count())
    }

    /// Retrieve all [`Tag`]s within the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn tags(&self) -> Result<Tags> {
        self.txn_wrap(|txn| txn.tags())
    }

    /// Retrieve the [`Tag`] matching the given [`TagId`]
    pub(crate) fn tag(&self, id: TagId) -> Result<Tag> {
        self.txn_wrap(|txn| txn.tag(id))
    }

    /// Retrieve the [`Tags`] matching the array of [`TagIds`]
    pub(crate) fn tags_by_ids(&self, ids: &TagIds) -> Result<Tags> {
        self.txn_wrap(|txn| txn.tags_by_ids(ids.inner()).map_err(Into::into))
    }

    /// Retrieve the [`Tag`] matching the given name
    pub(crate) fn tag_by_name<S: AsRef<str>>(&self, name: S) -> Result<Tag> {
        self.txn_wrap(|txn| txn.tag_by_name(name, false))
    }

    /// Retrieve the [`Tag`] matching the given name (ignoring case)
    pub(crate) fn tag_by_name_nocase<S: AsRef<str>>(&self, name: S) -> Result<Tag> {
        self.txn_wrap(|txn| txn.tag_by_name(name, true))
    }

    /// Retrieve the [`Tags`] matching the given names
    pub(crate) fn tags_by_names<S: AsRef<str>>(&self, names: &[S]) -> Result<Tags> {
        self.txn_wrap(|txn| txn.tags_by_names(names, false).map_err(Into::into))
    }

    /// Retrieve the [`Tags`] matching the given names (ignoring case)
    pub(crate) fn tags_by_names_nocase<S: AsRef<str>>(&self, names: &[S]) -> Result<Tags> {
        self.txn_wrap(|txn| txn.tags_by_names(names, true).map_err(Into::into))
    }

    // ========================= Pattern Matching =========================

    /// Retrieve the [`Tags`] matching the given `pcre` regex
    pub(crate) fn tags_by_pcre_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_pcre("name", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] matching the given `regex`
    pub(crate) fn tags_by_regex_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_regex("name", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] with the name matching the given `iregex`
    pub(crate) fn tags_by_iregex_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_regex("name", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] with the name matching the given `regex`
    pub(crate) fn tags_by_glob_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_glob("name", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] with the name matching the given `iglob`
    pub(crate) fn tags_by_iglob_name<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_glob("name", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] with the color matching the given `pcre` regex
    pub(crate) fn tags_by_pcre_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_regex("color", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] with the color matching the given `regex`
    pub(crate) fn tags_by_regex_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_regex("color", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] with the color matching the given `iregex`
    pub(crate) fn tags_by_iregex_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_regex("color", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] with the color matching the given `regex`
    pub(crate) fn tags_by_glob_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_glob("color", patt.as_ref()))
    }

    /// Retrieve the [`Tags`] with the color matching the given `iglob`
    pub(crate) fn tags_by_iglob_color<S: AsRef<str>>(&self, patt: S) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_glob("color", patt.as_ref()))
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a [`Tag`] into the database
    pub(crate) fn insert_tag<S: AsRef<str>>(&self, name: S, color: S) -> Result<Tag> {
        Tag::validate_name(&name)?;
        self.wrap_commit(|txn| {
            // TODO: Decide whether this should express an error
            // let color = parse_color(&color).unwrap_or(Color::BrightWhite);
            let color = parse_color(&color)?;

            txn.insert_tag(name, color)
        })
    }

    /// Update the [`Tag`] by changing its `name`
    pub(crate) fn update_tag_name<S: AsRef<str>>(&self, id: TagId, name: S) -> Result<Tag> {
        Tag::validate_name(&name)?;

        self.wrap_commit(|txn| txn.update_tag_name(id, name).map_err(Into::into))
    }

    /// Update the [`Tag`] by changing its `color`
    pub(crate) fn update_tag_color<S: AsRef<str>>(&self, id: TagId, color: S) -> Result<Tag> {
        self.wrap_commit(|txn| {
            // TODO: Decide whether this should express an error
            let color = parse_color(&color).unwrap_or(Color::BrightWhite);
            txn.update_tag_color(id, color).map_err(Into::into)
        })
    }

    /// Create a new [`Tag`] and apply it to an existing [`Tag`]
    pub(crate) fn copy_tag<S: AsRef<str>>(&self, source_id: TagId, name: S) -> Result<Tag> {
        Tag::validate_name(&name)?;

        self.wrap_commit(|txn| {
            let source_tag = txn.tag(source_id)?;
            let new_tag = txn.insert_tag(name, source_tag.color())?;
            txn.copy_filetags(source_id, new_tag.id())?;

            Ok(new_tag)
        })
    }

    /// Delete a [`Tag`] matching the given [`TagId`]
    pub(crate) fn delete_tag(&self, id: TagId) -> Result<()> {
        self.wrap_commit(|txn| txn.delete_tag(id).map_err(Into::into))
    }

    // ============================== Other ===============================
    // ====================================================================

    /// Show information about a [`Tag`]
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn tag_info(&self) -> Result<Vec<TagFileCnt>> {
        self.txn_wrap(|txn| txn.tag_information())
    }
}
