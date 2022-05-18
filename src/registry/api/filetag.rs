//! Interactions with the [`FileTag`] object

use super::super::{
    common::path::FsPath,
    sqlbuilder::Sort,
    transaction::Txn,
    types::{
        file::FileId,
        filetag::{FileTag, FileTags},
        tag::TagId,
        value::ValueId,
    },
    Error, Registry,
};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::path::Path;

impl Registry {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Check whether the given [`File`] has the given [`Tag`]
    ///
    /// [`File`]: ../types/file/struct.File.html
    /// [`Tag`]: ../types/tag/struct.Tag.html
    pub(crate) fn filetag_exists(&self, ft: &FileTag) -> Result<bool> {
        self.txn_wrap(|txn| txn.filetag_exists(ft).map_err(Into::into))
    }

    /// Retrieve the number of [`FileTag`]s within the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn filetag_count(&self) -> Result<u32> {
        self.txn_wrap(|txn| txn.select_filetag_count())
    }

    /// Retrieve all tracked [`FileTags`] within the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn filetags(&self) -> Result<FileTags> {
        self.txn_wrap(|txn| txn.select_filetags())
    }

    /// Retrieve the number of [`FileTag`]s matching a [`FileId`]
    pub(crate) fn filetag_count_by_fileid(&self, id: FileId) -> Result<u32> {
        self.txn_wrap(|txn| txn.select_filetag_count_by_fileid(id))
    }

    /// Retrieve the number of [`FileTag`]s matching a [`TagId`]
    pub(crate) fn filetag_count_by_tagid(&self, id: TagId) -> Result<u32> {
        self.txn_wrap(|txn| txn.select_filetag_count_by_tagid(id))
    }

    /// Retrieve all tracked [`FileTags`] matching a [`TagId`]
    pub(crate) fn filetags_by_tagid(&self, id: TagId) -> Result<FileTags> {
        self.txn_wrap(|txn| txn.select_filetags_by_tagid(id))
    }

    /// Retrieve the number of [`FileTag`]s matching a [`ValueId`]
    pub(crate) fn filetag_count_by_valueid(&self, id: ValueId) -> Result<u32> {
        self.txn_wrap(|txn| txn.select_filetag_count_by_valueid(id))
    }

    /// Retrieve the [`FileTags`] matching a [`ValueId`]
    pub(crate) fn filetags_by_valueid(&self, id: ValueId) -> Result<FileTags> {
        self.txn_wrap(|txn| txn.select_filetags_by_valueid(id))
    }

    /// Retrieve the [`FileTags`] matching a [`FileId`]
    pub(crate) fn filetags_by_fileid(&self, id: FileId) -> Result<FileTags> {
        self.txn_wrap(|txn| txn.select_filetags_by_fileid(id))
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Add a [`FileTag`] to the database
    pub(crate) fn insert_filetag(&self, ft: &FileTag) -> Result<FileTag> {
        self.wrap_commit(|txn| txn.insert_filetag(ft))
    }

    /// Remove a [`FileTag`] in the database
    pub(crate) fn delete_filetag(&self, fid: FileId, tid: TagId, vid: ValueId) -> Result<()> {
        self.wrap_commit(|txn| {
            let ft = FileTag::new(fid, tid, vid);
            let exists = txn.filetag_exists(&ft)?;

            txn.delete_filetag(&ft)?;
            self.delete_file_if_untagged(txn, ft.file_id())?;

            Ok(())
        })
    }

    /// Remove all [`FileTag`]s matching a given [`FileId`]
    pub(crate) fn delete_filetag_by_fileid(&self, id: FileId) -> Result<()> {
        self.wrap_commit(|txn| {
            txn.delete_filetag_by_fileid(id)?;
            self.delete_file_if_untagged(txn, id)?;

            Ok(())
        })
    }

    /// Remove all [`FileTag`]s matching a given [`TagId`]
    pub(crate) fn delete_filetag_by_tagid(&self, tx: &Txn, id: TagId) -> Result<()> {
        self.wrap_commit_by(tx, |txn| {
            let ftags = txn.select_filetags_by_tagid(id)?;
            txn.delete_filetag_by_tagid(id)?;
            self.delete_untagged_files(txn, &ftags.file_ids())?;

            Ok(())
        })
    }

    /// Remove all [`FileTag`]s matching a given [`ValueId`]
    pub(crate) fn delete_filetag_by_valueid(&self, id: ValueId) -> Result<()> {
        self.wrap_commit(|txn| {
            let ftags = txn.select_filetags_by_valueid(id)?;
            txn.delete_filetag_by_valueid(id)?;
            self.delete_untagged_files(txn, &ftags.file_ids())?;

            Ok(())
        })
    }

    /// Copy one [`FileTag`] to another
    pub(crate) fn copy_filetags(&self, src: TagId, dest: TagId) -> Result<()> {
        self.wrap_commit(|txn| txn.copy_filetags(src, dest))
    }
}
