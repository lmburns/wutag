//! Interactions with the [`Tag`] object

use super::super::{
    types::{
        file::{File, FileId},
        tag::{Tag, TagFileCnt, TagId, TagIds, Tags},
        value::ValueId,
    },
    Registry,
};
use crate::utils::color::parse_color;
use anyhow::{anyhow, Context, Result};
use colored::{Color, Colorize};

impl Registry {
    // ╭──────────────────────────────────────────────────────────╮
    // │                        Retrieving                        │
    // ╰──────────────────────────────────────────────────────────╯

    /// Retrieve the number of [`Tag`]s within the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn tag_count(&self) -> Result<u32> {
        self.txn_wrap(|txn| txn.select_tag_count())
    }

    /// Retrieve the number of files a given [`Tag`] is associated with
    pub(crate) fn tag_count_by_id(&self, id: TagId) -> Result<u32> {
        self.txn_wrap(|txn| txn.select_tag_count_by_id(id))
    }

    /// Select the maximum [`ID`] from [`Tag`]s
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn tag_max(&self) -> Result<u32> {
        self.txn_wrap(|txn| txn.select_tag_max())
    }

    /// Retrieve all [`Tag`]s within the database
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub(crate) fn tags(&self) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags())
    }

    /// Retrieve the [`Tag`] matching the given [`TagId`]
    pub(crate) fn tag(&self, id: TagId) -> Result<Tag> {
        self.txn_wrap(|txn| txn.select_tag(id))
    }

    /// Select [`Tags`] that are only connected to the given [`FileId`]
    pub(crate) fn unique_tags_by_file(&self, fid: FileId) -> Result<Tags> {
        self.wrap_commit(|txn| txn.select_unique_tags_by_file(fid))
    }

    /// Select [`Tags`] that are only connected to one [`File`]
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn unique_tags(&self) -> Result<Tags> {
        self.wrap_commit(|txn| txn.select_unique_tags())
    }

    /// Select all [`Tag`]s that are not associated with a [`Value`] or [`File`]
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn dangling_tags(&self) -> Result<Tags> {
        self.txn_wrap(|txn| txn.dangling_tags())
    }

    /// Retrieve the [`Tags`] matching the [`ValueId`]
    pub(crate) fn tags_by_valueid(&self, vid: ValueId) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_valueid(vid))
    }

    /// Retrieve the [`Tags`] matching the [`FileId`]
    pub(crate) fn tags_by_fileid(&self, fid: FileId) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_fileid(fid))
    }

    /// Retrieve the [`Tags`] matching the [`FileId`] and [`ValueId`]
    pub(crate) fn tags_by_fileid_valueid(&self, fid: FileId, vid: ValueId) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_fileid_valueid(fid, vid))
    }

    /// Retrieve the [`Tags`] matching the array of [`TagIds`]
    pub(crate) fn tags_by_ids(&self, ids: &TagIds) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_ids(ids.inner()).map_err(Into::into))
    }

    /// Retrieve the [`Tag`] matching the given name
    pub(crate) fn tag_by_name<S: AsRef<str>>(&self, name: S) -> Result<Tag> {
        self.txn_wrap(|txn| txn.select_tag_by_name(name, false))
    }

    /// Retrieve the [`Tag`] matching the given name (ignoring case)
    pub(crate) fn tag_by_name_nocase<S: AsRef<str>>(&self, name: S) -> Result<Tag> {
        self.txn_wrap(|txn| txn.select_tag_by_name(name, true))
    }

    /// Retrieve the [`Tags`] matching the given names
    pub(crate) fn tags_by_names<S: AsRef<str>>(&self, names: &[S]) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_names(names, false).map_err(Into::into))
    }

    /// Retrieve the [`Tags`] matching the given names (ignoring case)
    pub(crate) fn tags_by_names_nocase<S: AsRef<str>>(&self, names: &[S]) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_tags_by_names(names, true).map_err(Into::into))
    }

    /// Retrieve the [`Tags`] matching the given [`File`]
    pub(crate) fn tags_for_file(&self, file: &File) -> Result<Tags> {
        self.txn_wrap(|txn| txn.select_files_tags(file))
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                     Pattern Matching                     │
    // ╰──────────────────────────────────────────────────────────╯

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

    // ╭──────────────────────────────────────────────────────────╮
    // │                        Modifying                         │
    // ╰──────────────────────────────────────────────────────────╯

    /// Insert a [`Tag`] into the database
    pub(crate) fn insert_tag(&self, tag: &Tag) -> Result<Tag> {
        Tag::validate_name(&tag.name())?;
        self.wrap_commit(|txn| txn.insert_tag(tag.name(), tag.color()))
    }

    /// Update the [`Tag`] by changing its `name`
    pub(crate) fn update_tag_name<S: AsRef<str>>(&self, tag: &Tag, name: S) -> Result<Tag> {
        Tag::validate_name(&name)?;
        self.wrap_commit(|txn| txn.update_tag_name(tag.id(), name).map_err(Into::into))
    }

    /// Update the [`Tag`] by changing its `color`
    pub(crate) fn update_tag_color<S: AsRef<str>>(&self, id: TagId, color: S) -> Result<Tag> {
        self.wrap_commit(|txn| {
            let color = parse_color(&color).unwrap_or(Color::BrightWhite);
            txn.update_tag_color(id, color).map_err(Into::into)
        })
    }

    /// Create a new [`Tag`] and apply it to an existing [`Tag`]
    pub(crate) fn copy_tag<S: AsRef<str>>(&self, source_id: TagId, name: S) -> Result<Tag> {
        Tag::validate_name(&name)?;

        self.wrap_commit(|txn| {
            let source_tag = txn.select_tag(source_id)?;
            let new_tag = txn.insert_tag(name, source_tag.color())?;
            txn.copy_filetags(source_id, new_tag.id())?;

            Ok(new_tag)
        })
    }

    /// Delete a [`Tag`] matching the given [`TagId`]
    pub(crate) fn delete_tag(&self, id: TagId) -> Result<()> {
        self.wrap_commit(|txn| {
            self.delete_filetag_by_tagid(txn, id)?;
            txn.delete_tag(id).map_err(Into::into)
        })
    }

    /// Delete [`Tags`] that are not associated with any [`File`] or [`Tag`]
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn delete_dangling_tags(&self) -> Result<()> {
        self.wrap_commit(|txn| txn.delete_dangling_tags())
    }

    /// Delete all [`Tag`]s from the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn clear_tags(&self) -> Result<()> {
        self.wrap_commit(|txn| txn.clear_tags())
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                          Other                           │
    // ╰──────────────────────────────────────────────────────────╯

    /// Show information about a [`Tag`]
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn tag_info(&self) -> Result<Vec<TagFileCnt>> {
        self.txn_wrap(|txn| txn.tag_information())
    }
}
