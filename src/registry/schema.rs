//! The schema to setup the `Registry`

#![allow(unused)]

use super::{version::Version, Registry};
use anyhow::{Context, Result};
use rusqlite::params;

// ================== Initialization ==================

impl Registry<'_> {
    /// Create the `Tag` table
    ///
    /// This table contains information about tags
    pub(crate) fn create_tag_table(&self) -> Result<()> {
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS tag (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL
            )",
        )
        .context("failed to create table `tag`")?;

        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_tag_name
            ON tag(name)",
        )
        .context("failed to create index `idx_tag_name`")?;

        Ok(())
    }

    /// Create the file table
    ///
    /// This table contains many details about a single file
    pub(crate) fn create_file_table(&self) -> Result<()> {
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS file (
                id INTEGER PRIMARY KEY,
                directory TEXT NOT NULL,
                name TEXT NOT NULL,
                hash TEXT NOT NULL,
                mime TEXT NOT NULL,
                mod_time DATETIME NOT NULL,
                mode INTEGER NOT NULL,
                inode INTEGER NOT NULL,
                size INTEGER NOT NULL,
                is_dir BOOLEAN NOT NULL,
                CONSTRAINT con_file_path UNIQUE (directory, name)
            )",
        )
        .context("failed to create table `file`")?;

        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_file_hash
            ON file(fingerprint)`",
        )
        .context("failed to create index `idx_file_hash`")?;

        Ok(())
    }

    /// Create the `xattr` table
    ///
    /// This table contains values that tags can be set to. It is an extension
    /// of a tag. Many tags can have the same extended value, or have none at
    /// all
    pub(crate) fn create_value_table(&self) -> Result<()> {
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS xattr (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                CONSTRAINT con_xattr_name UNIQUE (name)
            )",
        )
        .context("failed to create table `values`")?;

        Ok(())
    }

    /// Create the `file_tag` table
    ///
    /// This table links files with their tags and the extended attributes that
    /// correspond to that tag
    pub(crate) fn create_file_tag_table(&self) -> Result<()> {
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS file_tag (
                file_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                xattr_id INTEGER NOT NULL,
                PRIMARY KEY (file_id, tag_id, xattr_id),
                FOREIGN KEY (file_id) REFERENCES file(id),
                FOREIGN KEY (tag_id) REFERENCES tag(id),
                FOREIGN KEY (xattr_id) REFERENCES xattr(id)
            )",
        )
        .context("failed to create table `file_tag`")?;

        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_file_tag_file_id
            ON file_tag(file_id)",
        )
        .context("failed to create index `idx_file_tag_file_id`")?;

        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_file_tag_tag_id
            ON file_tag(tag_id)",
        )
        .context("failed to create index `idx_file_tag_tag_id`")?;

        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_file_tag_xattr_id
            ON file_tag(xattr_id)",
        )
        .context("failed to create index `idx_file_tag_xattr_id`")?;

        Ok(())
    }

    /// Create the `impl` table
    ///
    /// An `impl` in this context is a tag that when set to a file, **implies**
    /// another tag
    pub(crate) fn create_impl_table(&self) -> Result<()> {
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS impl (
                tag_id INTEGER NOT NULL,
                value_id INTEGER NOT NULL,
                implied_tag_id INTEGER NOT NULL,
                implied_value_id INTEGER NOT NULL,
                PRIMARY KEY (tag_id, value_id, implied_tag_id, implied_value_id)
            )",
        )
        .context("failed to create table `impl`")?;

        Ok(())
    }

    /// Create the `query` table
    ///
    /// This table is used to query other tables
    pub(crate) fn create_query_table(&self) -> Result<()> {
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS query (
                sha BINARY(32) PRIMARY KEY,
                text TEXT NOT NULL
            )",
        )
        .context("failed to create table `query`")?;

        Ok(())
    }

    /// Create the `version` table
    ///
    /// This table is used to query other tables
    pub(crate) fn create_version_table(&self) -> Result<()> {
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS version (
                major NUMBER NOT NULL,
                minor NUMBER NOT NULL,
                patch NUMBER NOT NULL,
                PRIMARY KEY (major, minor, patch)
            )",
        )
        .context("failed to create table `version`")?;

        Ok(())
    }

    // ===================== Version ======================

    /// Insert the latest version into the database
    pub(crate) fn insert_version(&self) -> Result<()> {
        let v = Version::version().context("failed to get current version")?;

        self.insert(
            "INSERT INTO version (major, minor, patch)
                VALUES (?1, ?2, ?3)",
            params![v.major(), v.minor(), v.patch()],
        )
        .context("failed to insert version into `version` table")?;

        Ok(())
    }

    /// Get the current version of the database
    pub(crate) fn get_current_version(&self) -> Result<Version> {
        let res = self
            .select("SELECT * from version", params![], |row| {
                Ok(Version::new(row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .context("failed to query row")?;

        Ok(res)
    }

    /// Update the current version of the database
    pub(crate) fn update_current_version(&self) -> Result<()> {
        let v = Version::version().context("failed to get current version")?;

        self.execute(
            "UPDATE version SET major = ?1, minor = ?2, patch = ?3",
            params![v.major(), v.minor(), v.patch()],
        )
        .context("failed to update current version")?;

        Ok(())
    }
}
