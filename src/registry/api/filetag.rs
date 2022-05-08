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
                if let Ok(ftags) = self.filetags_by_fileid(id, false) {
                    return Ok(ftags.len() as u32);
                }

                Ok(0)
            }
        })
    }

    /// Retrieve the number of [`FileTag`]s matching a [`TagId`]
    pub(crate) fn filetag_count_by_tagid(&self, id: TagId, explicit: bool) -> Result<u32> {
        self.txn_wrap(|txn| {
            if explicit {
                txn.select_filetag_count_by_tagid(id)
            } else {
                if let Ok(ftags) = self.filetags_by_fileid(id, false) {
                    return Ok(ftags.len() as u32);
                }

                Ok(0)
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
                Ok(self.add_implied_filetags(txn, ftags)?)
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
                Ok(self.add_implied_filetags(txn, ftags)?)
            }
        })
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Add a [`FileTag`] to the database
    pub(crate) fn insert_filetag(&self, txn: &Txn, ft: &FileTag) -> Result<FileTag> {
        self.wrap_commit(txn, |t| t.insert_filetag(ft))
    }

    /// Remove a [`FileTag`] in the database
    pub(crate) fn delete_filetag(&self, txn: &Txn, ft: &FileTag) -> Result<()> {
        self.wrap_commit(txn, |t| {
            let exists = self.filetag_exists(ft)?;

            if !exists {
                return Err(anyhow!(
                    "the given `FileTag` does not exist:\n\t- File: {}, Tag: {}, Value: {}",
                    ft.file_id(),
                    ft.tag_id(),
                    ft.value_id()
                ));
            }

            t.delete_filetag(ft)?;
            self.delete_file_if_untagged(t, ft.file_id())?;

            Ok(())
        })
    }

    /// Remove all [`FileTag`]s matching a given [`FileId`]
    pub(crate) fn delete_filetag_by_fileid(&self, txn: &Txn, id: FileId) -> Result<()> {
        self.wrap_commit(txn, |t| {
            t.delete_filetag_by_fileid(id)?;
            self.delete_file_if_untagged(t, id)?;

            Ok(())
        })
    }

    /// Remove all [`FileTag`]s matching a given [`TagId`]
    pub(crate) fn delete_filetag_by_tagid(&self, txn: &Txn, id: TagId) -> Result<()> {
        self.wrap_commit(txn, |t| {
            let ftags = t.select_filetags_by_tagid(id)?;
            t.delete_filetag_by_tagid(id)?;
            self.delete_untagged_files(t, &ftags.file_ids())?;

            Ok(())
        })
    }

    /// Remove all [`FileTag`]s matching a given [`ValueId`]
    pub(crate) fn delete_filetag_by_valueid(&self, txn: &Txn, id: ValueId) -> Result<()> {
        self.wrap_commit(txn, |t| {
            let ftags = t.select_filetags_by_valueid(id)?;
            t.delete_filetag_by_valueid(id)?;
            self.delete_untagged_files(t, &ftags.file_ids())?;

            Ok(())
        })
    }

    /// Copy one [`FileTag`] to another
    pub(crate) fn copy_filetags(&self, txn: &Txn, src: TagId, dest: TagId) -> Result<()> {
        self.wrap_commit(txn, |t| t.copy_filetags(src, dest))
    }

    /// Add [`FileTags`] that are implied
    fn add_implied_filetags(&self, txn: &Txn, mut ftags: FileTags) -> Result<FileTags> {
        let mut tags = vec![];

        for f in ftags.iter() {
            let implications = self.implications_for(txn, &[f.to_tag_value_combo()])?;

            for implication in implications.iter() {
                let implied_ftag = ftags
                    .iter()
                    .find(|ftag| {
                        ftag.file_id() == f.file_id()
                            && ftag.tag_id() == implication.implied_tag().id()
                            && ftag.value_id() == implication.implied_val().id()
                    })
                    .copied();

                if let Some(mut iftag) = implied_ftag {
                    iftag.implicit = true;
                    tags.push(iftag);
                } else {
                    let implied = FileTag::new(f.file_id(), f.tag_id(), f.value_id());
                    tags.push(implied);
                }
            }
        }

        Ok(FileTags::from(tags))
    }
}
