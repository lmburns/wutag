//! Interactions with the [`FileTag`] object

use super::super::{
    common::path::FsPath,
    sqlbuilder::Sort,
    types::{
        file::FileId,
        filetag::{FileTag, FileTags},
        tag::TagId,
        value::ValueId,
    },
    Registry,
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
        let txn = self.txn()?;
        txn.filetag_exists(ft)
    }

    /// Retrieve the number of [`FileTag`]s within the database
    pub(crate) fn filetag_count(&self) -> Result<u32> {
        let txn = self.txn()?;
        txn.select_filetag_count()
    }

    /// Retrieve all tracked [`FileTags`] within the database
    pub(crate) fn filetags(&self) -> Result<FileTags> {
        let txn = self.txn()?;
        txn.select_filetags()
    }

    /// Retrieve the number of [`FileTag`]s matching a [`FileId`]
    pub(crate) fn filetag_count_by_fileid(&self, id: FileId, explicit: bool) -> Result<u32> {
        let txn = self.txn()?;

        if explicit {
            txn.select_filetag_count_by_fileid(id)
        } else {
            // let ftags = self.filetags_by_fileid();
            // if ftags.is_err() {
            //     Ok(0)
            // } else {
            //     Ok(ftags.len())
            // }
            todo!()
        }
    }

    /// Retrieve the number of [`FileTag`]s matching a [`TagId`]
    pub(crate) fn filetag_count_by_tagid(&self, id: TagId, explicit: bool) -> Result<u32> {
        let txn = self.txn()?;

        if explicit {
            txn.select_filetag_count_by_tagid(id)
        } else {
            // let ftags = self.filetags_by_tagid();
            // if ftags.is_err() {
            //     Ok(0)
            // } else {
            //     Ok(ftags.len())
            // }
            todo!()
        }
    }

    /// Retrieve all tracked [`FileTags`] matching a [`TagId`]
    pub(crate) fn filetags_by_tagid(&self, id: TagId, explicit: bool) -> Result<FileTags> {
        let txn = self.txn()?;
        let ftags = txn.select_filetags_by_tagid(id)?;

        if explicit {
            Ok(ftags)
        } else {
            // add_implied_filetags
            todo!()
        }
    }

    /// Retrieve the number of [`FileTag`]s matching a [`ValueId`]
    pub(crate) fn filetag_count_by_valueid(&self, id: ValueId) -> Result<u32> {
        let txn = self.txn()?;
        txn.select_filetag_count_by_valueid(id)
    }

    /// Retrieve the [`FileTags`] matching a [`ValueId`]
    pub(crate) fn filetags_by_valueid(&self, id: ValueId) -> Result<FileTags> {
        let txn = self.txn()?;
        txn.select_filetags_by_valueid(id)
    }

    /// Retrieve the [`FileTags`] matching a [`FileId`]
    pub(crate) fn filetags_by_fileid(&self, id: FileId, explicit: bool) -> Result<FileTags> {
        let txn = self.txn()?;
        let ftags = txn.select_filetags_by_fileid(id)?;

        if explicit {
            Ok(ftags)
        } else {
            // add_implied_filetags
            todo!()
        }
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Add a [`FileTag`] to the database
    pub(crate) fn insert_filetag(&self, ft: FileTag) -> Result<FileTag> {
        let txn = self.txn()?;
        txn.insert_filetag(&ft)
    }

    /// Remove a [`FileTag`] in the database
    pub(crate) fn delete_filetag(&self, ft: FileTag) -> Result<()> {
        let txn = self.txn()?;
        let exists = self.filetag_exists(&ft)?;

        if !exists {
            return Err(anyhow!(
                "the given `FileTag` does not exist:\n\t- File: {}, Tag: {}, Value: {}",
                ft.file_id(),
                ft.tag_id(),
                ft.value_id()
            ));
        }

        txn.delete_filetag(&ft)?;
        self.delete_file_if_untagged(ft.file_id())?;

        Ok(())
    }

    /// Remove all [`FileTag`]s matching a given [`FileId`]
    pub(crate) fn delete_filetag_by_fileid(&self, id: FileId) -> Result<()> {
        let txn = self.txn()?;
        txn.delete_filetag_by_fileid(id)?;
        self.delete_file_if_untagged(id)?;

        Ok(())
    }

    /// Remove all [`FileTag`]s matching a given [`FileId`]
    pub(crate) fn delete_filetag_by_tagid(&self, id: TagId) -> Result<()> {
        let txn = self.txn()?;
        let ftags = txn.select_filetags_by_tagid(id)?;
        txn.delete_filetag_by_tagid(id)?;
        self.delete_untagged_files(&ftags.file_ids());

        Ok(())
    }
}
