//! Interactions with the [`File`] object

use std::path::Path;

use super::super::{
    common::path::FsPath,
    sqlbuilder::Sort,
    types::file::{File, FileId, Files},
    Registry,
};
use anyhow::{Context, Result};

impl Registry {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of [`File`]s within the database
    pub(crate) fn file_count(&self) -> Result<u32> {
        let txn = self.txn()?;
        txn.select_file_count()
    }

    /// Retrieve all tracked [`File]s within the database
    pub(crate) fn files(&self, sort: Option<Sort>) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files(sort)
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
        // let fspath: FsPath = path.as_ref().into();
        // let relative = fspath.relative()?;

        txn.select_files_by_directory(path.as_ref().to_string_lossy(), false)
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

    /// Retrieve the number of [`File`]s matching a specific `hash`
    pub(crate) fn file_count_by_hash<S: AsRef<str>>(&self, fp: S) -> Result<u32> {
        let txn = self.txn()?;
        txn.select_file_count_by_hash(fp)
    }

    /// Retrieve all [`File`]s matching a specific `hash`
    pub(crate) fn files_by_hash<S: AsRef<str>>(&self, fp: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_hash(fp)
    }

    /// Retrieve all [`File`]s matching a specific `MimeType`
    pub(crate) fn files_by_mime<S: AsRef<str>>(&self, mime: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_mime(mime)
    }

    // TODO: Parse datetime

    /// Retrieve all [`File`]s matching a specific `mtime`
    pub(crate) fn files_by_mtime<S: AsRef<str>>(&self, mtime: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_mtime(mtime)
    }

    /// Retrieve all [`File`]s matching a specific `ctime`
    pub(crate) fn files_by_ctime<S: AsRef<str>>(&self, ctime: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_ctime(ctime)
    }

    /// Retrieve all [`File`]s matching a specific `mode`
    pub(crate) fn files_by_mode<S: AsRef<str>>(&self, mode: S) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_mode(mode)
    }

    /// Retrieve all [`File`]s matching a specific `inode`
    pub(crate) fn files_by_inode(&self, inode: u64) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_inode(inode)
    }

    // TODO: Parse file sizes

    /// Retrieve all [`File`]s matching a specific `size`
    pub(crate) fn files_by_size(&self, size: u64) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_by_size(size)
    }

    /// Retrieve all [`File`]s that are untagged
    pub(crate) fn files_untagged(&self) -> Result<Files> {
        let txn = self.txn()?;
        txn.select_files_untagged()
    }

    // ============================= Modifying ============================
    // ====================================================================
}
