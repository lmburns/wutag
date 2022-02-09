//! Interactions with the [`File`] object

use super::super::{
    common::path::FsPath,
    sqlbuilder::Sort,
    types::file::{File, FileId, FileIds, Files},
    Registry,
};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::path::Path;

impl Registry {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of [`File`]s within the database
    pub(crate) fn file_count(&self) -> Result<u32> {
        let txn = self.txn()?;
        txn.select_file_count()
    }

    /// Retrieve all tracked [`Files`] within the database
    pub(crate) fn files(&self, sort: Option<Sort>) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files(sort)
    }

    /// Retrieve all tracked [`Files`] that are directories
    pub(crate) fn directories(&self) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_directories()
    }

    /// Retrieve a specific [`File`] within the database
    pub(crate) fn file(&self, id: FileId) -> Result<File> {
        let txn = self.txn()?;
        txn.select_file(id)
    }

    /// Retrieve a [`File`] that matches the given path
    pub(crate) fn file_by_path<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let txn = self.txn()?;
        txn.select_file_by_path(path)
    }

    /// Retrieve all [`File`]s that reside in the given directory
    pub(crate) fn files_by_directory<P: AsRef<Path>>(&self, path: P) -> Result<Files> {
        let txn = self.txn()?;
        let path = path.as_ref();

        // TODO: Add relative check

        if !path.is_dir() {
            return Err(anyhow!(
                "{} is not a directory",
                path.display().to_string().red()
            ));
        }

        txn.select_files_by_directory(path.to_string_lossy(), false)
    }

    /// Retrieve all [`File`]s that reside in the given directory
    pub(crate) fn files_by_directories<P: AsRef<Path>>(&self, paths: &[P]) -> Result<Files> {
        let txn = self.txn()?;
        // let fspath: FsPath = path.as_ref().into();
        // let relative = fspath.relative()?;

        let mut files = Files::empty();
        for p in paths.iter().map(AsRef::as_ref).collect::<Vec<_>>() {
            files.combine(&txn.select_files_by_directory(p.to_string_lossy(), false)?);
        }

        Ok(files)
    }

    /// Retrieve the number of [`Files`] matching a specific `hash`
    pub(crate) fn file_count_by_hash<S: AsRef<str>>(&self, fp: S) -> Result<u32> {
        let txn = self.txn()?;
        txn.select_file_count_by_hash(fp)
    }

    /// Retrieve all [`Files`] matching a specific `hash`
    pub(crate) fn files_by_hash<S: AsRef<str>>(&self, fp: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_hash(fp)
    }

    /// Retrieve all [`Files`] matching a specific `MimeType`
    pub(crate) fn files_by_mime<S: AsRef<str>>(&self, mime: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_mime(mime)
    }

    // TODO: Parse datetime

    /// Retrieve all [`Files`] matching a specific `mtime`
    pub(crate) fn files_by_mtime<S: AsRef<str>>(&self, mtime: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_mtime(mtime)
    }

    /// Retrieve all [`Files`] matching a specific `ctime`
    pub(crate) fn files_by_ctime<S: AsRef<str>>(&self, ctime: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_ctime(ctime)
    }

    /// Retrieve all [`Files`] matching a specific `mode`
    pub(crate) fn files_by_mode<S: AsRef<str>>(&self, mode: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_mode(mode)
    }

    /// Retrieve all [`Files`] matching a specific `inode`
    pub(crate) fn files_by_inode(&self, inode: u64) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_inode(inode)
    }

    /// Retrieve all [`Files`] matching a specific `inode`
    pub(crate) fn files_by_links(&self, links: u64) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_links(links)
    }

    /// Retrieve all [`Files`] matching a specific `UID`
    pub(crate) fn files_by_uid(&self, uid: u64) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_uid(uid)
    }

    /// Retrieve all [`Files`] matching a specific `GID`
    pub(crate) fn files_by_gid(&self, gid: u64) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_gid(gid)
    }

    // TODO: Parse file sizes

    /// Retrieve all [`Files`] matching a specific `size`
    pub(crate) fn files_by_size(&self, size: u64) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_size(size)
    }

    #[cfg(all(
        feature = "file-flags",
        target_family = "unix",
        not(target_os = "macos")
    ))]
    /// Retrieve all [`Files`] that have given flags
    pub(crate) fn files_by_flags<S: AsRef<str>>(&self, given: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_flag(given)
    }

    /// Retrieve all [`Files`] that are untagged
    pub(crate) fn files_untagged(&self) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_untagged()
    }

    /// Retrieve the number of files that match a given query and path
    #[allow(clippy::unused_self)]
    pub(crate) fn files_count_for_query(&self) -> Result<u32> {
        todo!()
    }

    /// Retrieve all [`Files`] that match a given query
    #[allow(clippy::unused_self)]
    pub(crate) fn files_by_query(&self) -> Result<Files> {
        todo!()
    }

    /// Retrieve all [`Files`] that are duplicates in the database
    pub(crate) fn duplicate_files(&self) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_duplicates()
    }

    // ========================= Pattern Matching =========================

    /// Retrieve all [`Files`] that match a given `regex`
    pub(crate) fn files_by_regex(&self, column: &str, patt: &str) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_regex(column, patt)
    }

    /// Retrieve all [`Files`] that match a given `iregex`
    pub(crate) fn files_by_iregex(&self, column: &str, patt: &str) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_iregex(column, patt)
    }

    /// Retrieve all [`Files`] that match a given `glob`
    pub(crate) fn files_by_glob(&self, column: &str, patt: &str) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_glob(column, patt)
    }

    /// Retrieve all [`Files`] that match a given `iglob`
    pub(crate) fn files_by_iglob(&self, column: &str, patt: &str) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_iglob(column, patt)
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Add a [`File`] to the database
    pub(crate) fn insert_file<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let txn = self.txn()?;
        txn.insert_file(path)
    }

    /// Update a [`File`]'s information in the database
    pub(crate) fn update_file<P: AsRef<Path>>(&self, id: FileId, path: P) -> Result<File> {
        let txn = self.txn()?;
        Ok(txn.update_file(id, path)?)
    }

    /// Remove a [`File`] from the database
    pub(crate) fn delete_file(&self, id: FileId) -> Result<()> {
        let txn = self.txn()?;
        Ok(txn.delete_file(id)?)
    }

    /// Remove a [`File`] from the database if it is not tagged
    pub(crate) fn delete_file_if_untagged(&self, id: FileId) -> Result<()> {
        let txn = self.txn()?;
        let count = txn.select_filetag_count_by_fileid(id)?;

        if count == 0 {
            txn.delete_file(id)?;
        }

        Ok(())
    }

    /// Remove an array of [`File`]s from the database if they're untagged
    pub(crate) fn delete_untagged_files(&self, ids: &FileIds) -> Result<()> {
        let txn = self.txn()?;
        txn.delete_files_untagged(ids)
    }
}
