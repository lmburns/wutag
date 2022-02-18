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
        self.txn_wrap(|txn| txn.filetag_exists(ft))
    }

    /// Retrieve the number of [`FileTag`]s within the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn filetag_count2(&self) -> Result<u32> {
        self.txn_wrap(|txn| txn.select_filetag_count())
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
    pub(crate) fn filetag_count_by_fileid(&self, id: FileId, explicit: bool) -> Result<u32> {
        self.txn_wrap(|txn| {
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
        })
    }

    /// Retrieve the number of [`FileTag`]s matching a [`TagId`]
    pub(crate) fn filetag_count_by_tagid(&self, id: TagId, explicit: bool) -> Result<u32> {
        self.txn_wrap(|txn| {
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
        })
    }

    /// Retrieve all tracked [`FileTags`] matching a [`TagId`]
    pub(crate) fn filetags_by_tagid(&self, id: TagId, explicit: bool) -> Result<FileTags> {
        self.txn_wrap(|txn| {
            let ftags = txn.select_filetags_by_tagid(id)?;

            if explicit {
                Ok(ftags)
            } else {
                // add_implied_filetags
                todo!()
            }
        })
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
    pub(crate) fn filetags_by_fileid(&self, id: FileId, explicit: bool) -> Result<FileTags> {
        self.txn_wrap(|txn| {
            let ftags = txn.select_filetags_by_fileid(id)?;

            if explicit {
                Ok(ftags)
            } else {
                // add_implied_filetags
                todo!()
            }
        })
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Add a [`FileTag`] to the database
    pub(crate) fn insert_filetag(&self, ft: FileTag) -> Result<FileTag> {
        self.wrap_commit(|txn| txn.insert_filetag(&ft))
    }

    /// Remove a [`FileTag`] in the database
    pub(crate) fn delete_filetag(&self, ft: FileTag) -> Result<()> {
        self.wrap_commit(|txn| {
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
        })
    }

    /// Remove all [`FileTag`]s matching a given [`FileId`]
    pub(crate) fn delete_filetag_by_fileid(&self, id: FileId) -> Result<()> {
        self.wrap_commit(|txn| {
            txn.delete_filetag_by_fileid(id)?;
            self.delete_file_if_untagged(id)?;

            Ok(())
        })
    }

    /// Remove all [`FileTag`]s matching a given [`TagId`]
    pub(crate) fn delete_filetag_by_tagid(&self, id: TagId) -> Result<()> {
        self.wrap_commit(|txn| {
            let ftags = txn.select_filetags_by_tagid(id)?;
            txn.delete_filetag_by_tagid(id)?;
            self.delete_untagged_files(&ftags.file_ids())?;

            Ok(())
        })
    }

    /// Remove all [`FileTag`]s matching a given [`ValueId`]
    pub(crate) fn delete_filetag_by_valueid(&self, id: ValueId) -> Result<()> {
        self.wrap_commit(|txn| {
            let ftags = txn.select_filetags_by_valueid(id)?;
            txn.delete_filetag_by_valueid(id)?;
            self.delete_untagged_files(&ftags.file_ids())?;

            Ok(())
        })
    }

    /// Copy one [`FileTag`] to another
    pub(crate) fn copy_filetags(&self, src: TagId, dest: TagId) -> Result<()> {
        self.wrap_commit(|txn| txn.copy_filetags(src, dest))
    }

    // fn add_implied_filetags(&self, mut ftags: FileTags) -> Result<FileTags> {
    //     for f in ftags.iter() {
    //         let implications =
    // self.implications_for(&[&f.to_tag_value_combo()])?;
    //
    //         for implication in implications.iter() {
    //             let mut implied_ftag = ftags
    //                 .filter(|ftag| {
    //                     ftag.file_id() == f.file_id()
    //                         && ftag.tag_id() == implication.implied_tag().id()
    //                         && ftag.value_id() == implication.implied_val().id()
    //                 })
    //                 .first();
    //
    //             if let Some(ref mut iftag) = implied_ftag {
    //                 // Does nothing?
    //                 iftag.implicit = true;
    //             } else {
    //                 let implied_ftag = FileTag::new(f.file_id(), f.tag_id(),
    // f.value_id());                 ftags.push(implied_ftag);
    //             }
    //         }
    //     }
    //
    //     Ok(ftags)
    // }
}
