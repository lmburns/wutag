//! The schema to setup the `Registry`

#![allow(unused)]

use super::{common::version::Version, Registry};
use crate::fail;
use anyhow::{Context, Result};
use rusqlite::params;
use tern::t;

// ================== Initialization ==================

impl Registry {
    /// Create the `Tag` table
    ///
    /// This table contains information about tags
    pub(crate) fn create_tag_table(&self) -> Result<()> {
        log::debug!("creating TABLE(tag)");
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS tag (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL
            )",
        )
        .context(fail!("creating TABLE(tag)"))?;

        log::debug!("creating INDEX(idx_tag_name)");
        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_tag_name
            ON tag(name, color)",
        )
        .context(fail!("creating INDEX(idx_tag_name)"))?;

        Ok(())
    }

    /// Create the file table
    ///
    /// This table contains many details about a single file
    pub(crate) fn create_file_table(&self) -> Result<()> {
        log::debug!("creating TABLE(file)");
        self.exec_no_params(&format!(
            "CREATE TABLE IF NOT EXISTS file (
                id INTEGER PRIMARY KEY,
                directory TEXT NOT NULL,
                name TEXT NOT NULL,
                hash TEXT NOT NULL,
                mime TEXT NOT NULL,
                mtime DATETIME NOT NULL,
                ctime DATETIME NOT NULL,
                mode INTEGER NOT NULL,
                inode INTEGER NOT NULL,
                links INTEGER NOT NULL,
                uid INTEGER NOT NULL,
                gid INTEGER NOT NULL,
                size INTEGER NOT NULL,
                is_dir BOOLEAN NOT NULL,
                is_symlink BOOLEAN NOT NULL,
                {}
                CONSTRAINT con_file_path UNIQUE (directory, name)
            )",
            t!((cfg!(feature = "file-flags")) ? "e2pflags INTEGER NOT NULL," : ""),
        ))
        .context(fail!("creating TABLE(file)"))?;

        log::debug!("creating INDEX(idx_file_hash)");
        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_file_hash
            ON file(hash)",
        )
        .context(fail!("creating INDEX(idx_file_hash)"))?;

        Ok(())
    }

    /// Create the `value` table
    ///
    /// This table contains values that tags can be set to. It is an extension
    /// of a tag. Many tags can have the same extended value, or have none at
    /// all
    pub(crate) fn create_value_table(&self) -> Result<()> {
        log::debug!("creating TABLE(value)");
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS value (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                CONSTRAINT con_value_name UNIQUE (name)
            )",
        )
        .context(fail!("creating TABLE(value)"))?;

        Ok(())
    }

    /// Create the `file_tag` table
    ///
    /// This table links files with their tags and the extended attributes that
    /// correspond to that tag
    pub(crate) fn create_file_tag_table(&self) -> Result<()> {
        log::debug!("creating TABLE(file_tag)");
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS file_tag (
                file_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                value_id INTEGER NOT NULL,
                PRIMARY KEY (file_id, tag_id, value_id),
                FOREIGN KEY (file_id) REFERENCES file(id),
                FOREIGN KEY (tag_id) REFERENCES tag(id),
                FOREIGN KEY (value_id) REFERENCES value(id)
            )",
        )
        .context(fail!("creating TABLE(file_tag)"))?;

        log::debug!("creating INDEX(idx_file_tag_file_id)");
        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_file_tag_file_id
            ON file_tag(file_id)",
        )
        .context(fail!("creating INDEX(idx_file_tag_file_id)"))?;

        log::debug!("creating INDEX(idx_file_tag_tag_id)");
        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_file_tag_tag_id
            ON file_tag(tag_id)",
        )
        .context(fail!("creating INDEX(idx_file_tag_tag_id)"))?;

        log::debug!("creating INDEX(idx_file_tag_value_id)");
        self.exec_no_params(
            "CREATE INDEX IF NOT EXISTS idx_file_tag_value_id
            ON file_tag(value_id)",
        )
        .context(fail!("creating INDEX(idx_file_tag_value_id)"))?;

        Ok(())
    }

    /// Create the `query` table
    ///
    /// This table is used to query other tables
    pub(crate) fn create_query_table(&self) -> Result<()> {
        log::debug!("creating TABLE(query)");
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS query (
                text TEXT PRIMARY KEY
            )",
        )
        .context(fail!("creating TABLE(query)"))?;

        Ok(())
    }

    // TODO:
    /// Create the `tracker` table
    ///
    /// This table is used to track operations performed on files and tags. It
    /// is used in combination with the `checkpoint` table
    ///
    ///  - `operation`: Add, update, delete
    ///  - `table`: file, tag, file_tag, etc
    ///  - `operation_id`: id of action from the `table`
    ///  - `previous`: previous action
    pub(crate) fn create_tracker_table(&self) -> Result<()> {
        log::debug!("creating TABLE(tracker)");
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS tracker (
                tracker_id INTEGER PRIMARY KEY,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                table TEXT NOT NULL,
                operation TEXT NOT NULL,
                operation_id TEXT NOT NULL,
                previous TEXT
            )",
        )
        .context(fail!("creating TABLE(tracker)"))?;
        Ok(())
    }

    // TODO:
    /// Create the `checkpoint` table
    ///
    /// This table is used to log points where actions can be undone and
    /// reverted back to
    pub(crate) fn create_checkpoint_table(&self) -> Result<()> {
        log::debug!("creating TABLE(checkpoint)");
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS checkpoint (
                checkpoint_id INTEGER PRIMARY KEY,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                tracker_id INTEGER NOT NULL,
                description TEXT NOT NULL
            )",
        )
        .context(fail!("creating TABLE(checkpoint)"))?;
        Ok(())
    }

    /// Create the `version` table
    ///
    /// This table is used to query other tables
    pub(crate) fn create_version_table(&self) -> Result<()> {
        log::debug!("creating TABLE(version)");
        self.exec_no_params(
            "CREATE TABLE IF NOT EXISTS version (
                major NUMBER NOT NULL,
                minor NUMBER NOT NULL,
                patch NUMBER NOT NULL,
                PRIMARY KEY (major, minor, patch)
            )",
        )
        .context(fail!("creating TABLE(version)"))?;

        Ok(())
    }

    // ============================= Version ==============================
    // ====================================================================

    /// Insert the latest version into the database
    pub(crate) fn insert_version(&self) -> Result<()> {
        let v = Version::build().context(fail!("querying for Version"))?;
        log::debug!("inserting Version({})", v);

        self.insert(
            "INSERT INTO version (major, minor, patch)
                VALUES (?1, ?2, ?3)",
            params![v.major(), v.minor(), v.patch()],
        )
        .context(fail!("inserting Version({})", v))?;

        Ok(())
    }

    /// Get the current version of the database
    pub(crate) fn get_current_version(&self) -> Result<Version> {
        log::debug!("querying for Version");
        let res = self
            .select("SELECT * from version", params![], |row| {
                Ok(Version::new(row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .context(fail!("querying for Version"))?;

        Ok(res)
    }

    /// Update the current version of the database
    pub(crate) fn update_current_version(&self) -> Result<()> {
        let v = Version::build().context(fail!("querying for Version"))?;
        log::debug!("updating Version: {}", v);

        self.execute(
            "UPDATE version SET major = ?1, minor = ?2, patch = ?3",
            params![v.major(), v.minor(), v.patch()],
        )
        .context(fail!("updating Version"))?;

        Ok(())
    }
}
